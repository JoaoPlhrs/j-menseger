//! Armazenamento local em SQLite.
//!
//! O celular guarda tudo em IndexedDB. No computador usamos SQLite com o mesmo
//! modelo de dados, porque o histórico aqui tende a ser grande e precisa de
//! consulta indexada por conversa e data, sem carregar tudo na memória.
//!
//! A chave da conta também fica aqui (em `kv`). Isso é uma fraqueza
//! conhecida e documentada: quem copiar este arquivo leva a conta junto. O
//! celular usa o cofre do Android; o equivalente no computador (cofre do
//! sistema) está na lista de pendências do README.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{de::DeserializeOwned, Serialize};
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("banco de dados: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("dado malformado: {0}")]
    Json(#[from] serde_json::Error),
}

type Result<T> = std::result::Result<T, StoreError>;

pub struct Store {
    conn: Connection,
}

/// Versão do formato do banco. Subir este número exige escrever a migração.
const SCHEMA_VERSION: i64 = 1;

impl Store {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open(path)?;
        Self::init(conn)
    }

    /// Banco em memória, para testes.
    pub fn in_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Self> {
        // WAL deixa a interface ler enquanto a rede escreve, sem travar.
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;

        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS kv (
                k TEXT PRIMARY KEY,
                v TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS contacts (
                pk        TEXT PRIMARY KEY,
                name      TEXT NOT NULL DEFAULT '',
                nick      TEXT NOT NULL DEFAULT '',
                note      TEXT NOT NULL DEFAULT '',
                pic       TEXT,
                tone      INTEGER NOT NULL DEFAULT 0,
                status    TEXT NOT NULL DEFAULT 'request',
                verified  INTEGER NOT NULL DEFAULT 0,
                relays    TEXT NOT NULL DEFAULT '[]',
                prof_sent INTEGER NOT NULL DEFAULT 0,
                added_at  INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS groups (
                lid      TEXT PRIMARY KEY,
                pk       TEXT NOT NULL,
                sk       TEXT NOT NULL,
                old_keys TEXT NOT NULL DEFAULT '[]',
                meta     TEXT NOT NULL,
                ver      INTEGER NOT NULL DEFAULT 1,
                pic      TEXT,
                pic_v    INTEGER NOT NULL DEFAULT 0,
                pics     TEXT NOT NULL DEFAULT '{}',
                gone     TEXT NOT NULL DEFAULT '[]',
                muted    INTEGER NOT NULL DEFAULT 0,
                left_at  INTEGER,
                joined_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS chats (
                key         TEXT PRIMARY KEY,
                kind        TEXT NOT NULL,
                peer        TEXT,
                lid         TEXT,
                channel     TEXT,
                last_ts     INTEGER NOT NULL DEFAULT 0,
                last_id     TEXT,
                last_from   TEXT,
                last_kind   TEXT,
                last_text   TEXT NOT NULL DEFAULT '',
                last_status TEXT,
                unread      INTEGER NOT NULL DEFAULT 0,
                ttl         INTEGER NOT NULL DEFAULT 0,
                pinned      INTEGER NOT NULL DEFAULT 0,
                muted       INTEGER NOT NULL DEFAULT 0,
                deleted     INTEGER NOT NULL DEFAULT 0,
                pending_read TEXT NOT NULL DEFAULT '[]'
            );

            CREATE TABLE IF NOT EXISTS msgs (
                id      TEXT PRIMARY KEY,
                chat    TEXT NOT NULL,
                author  TEXT NOT NULL,
                kind    TEXT NOT NULL,
                ts      INTEGER NOT NULL,
                body    TEXT NOT NULL,
                status  TEXT,
                exp     INTEGER,
                reply   TEXT
            );
            CREATE INDEX IF NOT EXISTS msgs_chat_ts ON msgs(chat, ts);
            CREATE INDEX IF NOT EXISTS msgs_exp ON msgs(exp) WHERE exp IS NOT NULL;

            CREATE TABLE IF NOT EXISTS media (
                mid   TEXT PRIMARY KEY,
                msg_id TEXT,
                chat  TEXT,
                mime  TEXT NOT NULL DEFAULT '',
                bytes BLOB
            );

            CREATE TABLE IF NOT EXISTS chunks (
                mid TEXT NOT NULL,
                i   INTEGER NOT NULL,
                d   TEXT NOT NULL,
                PRIMARY KEY (mid, i)
            );

            CREATE TABLE IF NOT EXISTS calls (
                id     TEXT PRIMARY KEY,
                peer   TEXT NOT NULL,
                dir    TEXT NOT NULL,
                video  INTEGER NOT NULL DEFAULT 0,
                ts     INTEGER NOT NULL,
                dur    INTEGER NOT NULL DEFAULT 0,
                status TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS calls_ts ON calls(ts);

            CREATE TABLE IF NOT EXISTS outbox (
                id     TEXT PRIMARY KEY,
                wraps  TEXT NOT NULL,
                relays TEXT NOT NULL DEFAULT '[]',
                at     INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS sounds (
                id    TEXT PRIMARY KEY,
                name  TEXT NOT NULL,
                emoji TEXT NOT NULL DEFAULT '🔊',
                dur   REAL NOT NULL DEFAULT 1.0,
                mime  TEXT NOT NULL,
                bytes BLOB NOT NULL,
                ts    INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS seen (
                id TEXT PRIMARY KEY,
                at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS seen_at ON seen(at);
            "#,
        )?;

        // Bancos criados antes das reações não têm a coluna: acrescenta.
        let tem_reacts: bool = conn
            .prepare("SELECT 1 FROM pragma_table_info('msgs') WHERE name = 'reacts'")?
            .exists([])?;
        if !tem_reacts {
            conn.execute_batch("ALTER TABLE msgs ADD COLUMN reacts TEXT NOT NULL DEFAULT '{}';")?;
        }

        // Banner e jogos favoritos dos contatos (JSON), em bancos antigos.
        let tem_extra: bool = conn
            .prepare("SELECT 1 FROM pragma_table_info('contacts') WHERE name = 'extra'")?
            .exists([])?;
        if !tem_extra && conn.prepare("SELECT 1 FROM pragma_table_info('contacts')")?.exists([])? {
            conn.execute_batch("ALTER TABLE contacts ADD COLUMN extra TEXT NOT NULL DEFAULT '{}';")?;
        }

        let found: i64 = conn
            .query_row("SELECT v FROM kv WHERE k = 'schema'", [], |r| {
                r.get::<_, String>(0)
            })
            .optional()?
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        if found == 0 {
            conn.execute(
                "INSERT OR REPLACE INTO kv (k, v) VALUES ('schema', ?1)",
                params![SCHEMA_VERSION.to_string()],
            )?;
        }

        Ok(Self { conn })
    }

    /* ---------------- valores soltos ---------------- */

    pub fn set_kv<T: Serialize>(&self, key: &str, value: &T) -> Result<()> {
        let json = serde_json::to_string(value)?;
        self.conn.execute(
            "INSERT INTO kv (k, v) VALUES (?1, ?2) ON CONFLICT(k) DO UPDATE SET v = ?2",
            params![key, json],
        )?;
        Ok(())
    }

    pub fn get_kv<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        let raw: Option<String> = self
            .conn
            .query_row("SELECT v FROM kv WHERE k = ?1", params![key], |r| r.get(0))
            .optional()?;
        match raw {
            Some(s) => Ok(Some(serde_json::from_str(&s)?)),
            None => Ok(None),
        }
    }

    /* ---------------- envelopes já vistos ---------------- */

    /// Marca um envelope como processado. Devolve `false` se já era conhecido.
    ///
    /// Como a mesma mensagem chega por vários servidores, sem isto ela
    /// apareceria repetida na conversa.
    pub fn mark_seen(&self, id: &str, now: i64) -> Result<bool> {
        let n = self.conn.execute(
            "INSERT OR IGNORE INTO seen (id, at) VALUES (?1, ?2)",
            params![id, now],
        )?;
        Ok(n > 0)
    }

    /// Descarta registros antigos de envelopes vistos.
    pub fn prune_seen(&self, older_than: i64) -> Result<usize> {
        Ok(self
            .conn
            .execute("DELETE FROM seen WHERE at < ?1", params![older_than])?)
    }

    /* ---------------- mensagens ---------------- */

    pub fn put_msg(&self, m: &StoredMsg) -> Result<bool> {
        let n = self.conn.execute(
            "INSERT OR IGNORE INTO msgs (id, chat, author, kind, ts, body, status, exp, reply, reacts)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![m.id, m.chat, m.author, m.kind, m.ts, m.body, m.status, m.exp, m.reply, m.reacts],
        )?;
        Ok(n > 0)
    }

    /// Regrava tipo, conteúdo, situação e reações de uma mensagem que já existe.
    pub fn update_msg(&self, m: &StoredMsg) -> Result<()> {
        self.conn.execute(
            "UPDATE msgs SET kind = ?2, body = ?3, status = ?4, reacts = ?5, exp = ?6 WHERE id = ?1",
            params![m.id, m.kind, m.body, m.status, m.reacts, m.exp],
        )?;
        Ok(())
    }

    /// A mensagem mais recente de uma conversa.
    pub fn last_msg(&self, chat: &str) -> Result<Option<StoredMsg>> {
        Ok(self.msgs_by_chat(chat, i64::MAX, 1)?.pop())
    }

    pub fn get_msg(&self, id: &str) -> Result<Option<StoredMsg>> {
        self.conn
            .query_row(
                "SELECT id, chat, author, kind, ts, body, status, exp, reply, reacts FROM msgs WHERE id = ?1",
                params![id],
                row_to_msg,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn set_msg_status(&self, id: &str, status: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE msgs SET status = ?2 WHERE id = ?1",
            params![id, status],
        )?;
        Ok(())
    }

    /// Página de mensagens de uma conversa, da mais antiga para a mais nova.
    pub fn msgs_by_chat(&self, chat: &str, before_ts: i64, limit: usize) -> Result<Vec<StoredMsg>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, chat, author, kind, ts, body, status, exp, reply, reacts
             FROM msgs WHERE chat = ?1 AND ts < ?2
             ORDER BY ts DESC LIMIT ?3",
        )?;
        let mut v: Vec<StoredMsg> = stmt
            .query_map(params![chat, before_ts, limit as i64], row_to_msg)?
            .collect::<std::result::Result<_, _>>()?;
        v.reverse();
        Ok(v)
    }

    /// Mensagens temporárias cujo prazo já passou.
    pub fn msgs_expired(&self, now: i64) -> Result<Vec<StoredMsg>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, chat, author, kind, ts, body, status, exp, reply, reacts
             FROM msgs WHERE exp IS NOT NULL AND exp <= ?1",
        )?;
        let rows: Vec<StoredMsg> = stmt
            .query_map(params![now], row_to_msg)?
            .collect::<std::result::Result<_, _>>()?;
        Ok(rows)
    }

    pub fn delete_msg(&self, id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM msgs WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// Apaga uma conversa inteira deste aparelho e devolve quantas sumiram.
    pub fn delete_chat_msgs(&self, chat: &str) -> Result<usize> {
        let n = self
            .conn
            .execute("DELETE FROM msgs WHERE chat = ?1", params![chat])?;
        self.conn
            .execute("DELETE FROM media WHERE chat = ?1", params![chat])?;
        Ok(n)
    }

    pub fn count_msgs(&self, chat: &str) -> Result<i64> {
        Ok(self.conn.query_row(
            "SELECT COUNT(*) FROM msgs WHERE chat = ?1",
            params![chat],
            |r| r.get(0),
        )?)
    }

    /* ---------------- mídia em blocos ---------------- */

    pub fn put_chunk(&self, mid: &str, i: u32, d: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO chunks (mid, i, d) VALUES (?1, ?2, ?3)",
            params![mid, i, d],
        )?;
        Ok(())
    }

    pub fn chunks_of(&self, mid: &str) -> Result<Vec<(u32, String)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT i, d FROM chunks WHERE mid = ?1 ORDER BY i")?;
        let rows: Vec<(u32, String)> = stmt
            .query_map(params![mid], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<std::result::Result<_, _>>()?;
        Ok(rows)
    }

    pub fn put_media(&self, mid: &str, msg_id: &str, chat: &str, mime: &str, bytes: &[u8]) -> Result<()> {
        self.conn.execute(
            "INSERT INTO media (mid, msg_id, chat, mime, bytes) VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(mid) DO UPDATE SET bytes = ?5, mime = ?4",
            params![mid, msg_id, chat, mime, bytes],
        )?;
        self.conn
            .execute("DELETE FROM chunks WHERE mid = ?1", params![mid])?;
        Ok(())
    }

    pub fn media_bytes(&self, mid: &str) -> Result<Option<Vec<u8>>> {
        self.conn
            .query_row(
                "SELECT bytes FROM media WHERE mid = ?1",
                params![mid],
                |r| r.get::<_, Option<Vec<u8>>>(0),
            )
            .optional()
            .map(|o| o.flatten())
            .map_err(Into::into)
    }

    /// Registra que uma mídia é esperada, antes de os blocos chegarem.
    pub fn put_media_pending(&self, mid: &str, msg_id: &str, chat: &str, mime: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO media (mid, msg_id, chat, mime, bytes) VALUES (?1, ?2, ?3, ?4, NULL)",
            params![mid, msg_id, chat, mime],
        )?;
        Ok(())
    }

    /// `(id da mensagem, conversa, já tem os bytes)` de uma mídia conhecida.
    pub fn media_record(&self, mid: &str) -> Result<Option<(Option<String>, Option<String>, bool)>> {
        self.conn
            .query_row(
                "SELECT msg_id, chat, bytes IS NOT NULL FROM media WHERE mid = ?1",
                params![mid],
                |r| Ok((r.get(0)?, r.get(1)?, r.get::<_, i64>(2)? != 0)),
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn delete_media(&self, mid: &str) -> Result<()> {
        self.conn.execute("DELETE FROM media WHERE mid = ?1", params![mid])?;
        self.conn.execute("DELETE FROM chunks WHERE mid = ?1", params![mid])?;
        Ok(())
    }

    /// Apaga fotos e áudios recebidos e enviados. Os textos ficam.
    pub fn clear_media(&self) -> Result<usize> {
        let n = self.conn.execute("DELETE FROM media", [])?;
        self.conn.execute("DELETE FROM chunks", [])?;
        Ok(n)
    }

    pub fn chunk_count(&self, mid: &str) -> Result<u32> {
        Ok(self.conn.query_row(
            "SELECT COUNT(*) FROM chunks WHERE mid = ?1",
            params![mid],
            |r| r.get::<_, i64>(0),
        )? as u32)
    }

    /// Espaço ocupado por fotos, áudios e sons, em bytes.
    pub fn media_size(&self) -> Result<i64> {
        Ok(self.conn.query_row(
            "SELECT COALESCE(SUM(LENGTH(bytes)), 0) FROM media",
            [],
            |r| r.get(0),
        )?)
    }

    /* ---------------- chamadas ---------------- */

    pub fn log_call(&self, c: &CallLog) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO calls (id, peer, dir, video, ts, dur, status) VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![c.id, c.peer, c.dir, c.video as i64, c.ts, c.dur, c.status],
        )?;
        Ok(())
    }

    pub fn calls(&self, limit: usize) -> Result<Vec<CallLog>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, peer, dir, video, ts, dur, status FROM calls ORDER BY ts DESC LIMIT ?1",
        )?;
        let rows: Vec<CallLog> = stmt
            .query_map(params![limit as i64], |r| {
                Ok(CallLog {
                    id: r.get(0)?,
                    peer: r.get(1)?,
                    dir: r.get(2)?,
                    video: r.get::<_, i64>(3)? != 0,
                    ts: r.get(4)?,
                    dur: r.get(5)?,
                    status: r.get(6)?,
                })
            })?
            .collect::<std::result::Result<_, _>>()?;
        Ok(rows)
    }

    pub fn clear_calls(&self) -> Result<()> {
        self.conn.execute("DELETE FROM calls", [])?;
        Ok(())
    }

    /* ---------------- sons próprios ---------------- */

    pub fn put_sound(&self, s: &SoundRecord) -> Result<()> {
        self.conn.execute(
            "INSERT INTO sounds (id, name, emoji, dur, mime, bytes, ts) VALUES (?1,?2,?3,?4,?5,?6,?7)
             ON CONFLICT(id) DO UPDATE SET name = ?2, emoji = ?3",
            params![s.id, s.name, s.emoji, s.dur, s.mime, s.bytes, s.ts],
        )?;
        Ok(())
    }

    pub fn sounds(&self) -> Result<Vec<SoundRecord>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, emoji, dur, mime, bytes, ts FROM sounds ORDER BY ts")?;
        let rows: Vec<SoundRecord> = stmt
            .query_map([], |r| {
                Ok(SoundRecord {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    emoji: r.get(2)?,
                    dur: r.get(3)?,
                    mime: r.get(4)?,
                    bytes: r.get(5)?,
                    ts: r.get(6)?,
                })
            })?
            .collect::<std::result::Result<_, _>>()?;
        Ok(rows)
    }

    pub fn delete_sound(&self, id: &str) -> Result<()> {
        self.conn.execute("DELETE FROM sounds WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// Tamanho do arquivo do banco, em bytes.
    pub fn db_size(&self) -> Result<i64> {
        let pages: i64 = self.conn.query_row("PRAGMA page_count", [], |r| r.get(0))?;
        let size: i64 = self.conn.query_row("PRAGMA page_size", [], |r| r.get(0))?;
        Ok(pages * size)
    }

    /* ---------------- fila de envio ---------------- */

    pub fn queue_outbox(&self, id: &str, wraps_json: &str, relays_json: &str, at: i64) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO outbox (id, wraps, relays, at) VALUES (?1, ?2, ?3, ?4)",
            params![id, wraps_json, relays_json, at],
        )?;
        Ok(())
    }

    pub fn outbox(&self) -> Result<Vec<(String, String, String)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, wraps, relays FROM outbox ORDER BY at")?;
        let rows: Vec<(String, String, String)> = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
            .collect::<std::result::Result<_, _>>()?;
        Ok(rows)
    }

    pub fn drop_outbox(&self, id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM outbox WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// Apaga tudo: usado em "sair e apagar deste aparelho".
    pub fn wipe(&self) -> Result<()> {
        self.conn.execute_batch(
            "DELETE FROM msgs; DELETE FROM media; DELETE FROM chunks; DELETE FROM chats;
             DELETE FROM contacts; DELETE FROM groups; DELETE FROM calls; DELETE FROM outbox;
             DELETE FROM sounds; DELETE FROM seen; DELETE FROM kv;",
        )?;
        Ok(())
    }

    pub fn raw(&self) -> &Connection {
        &self.conn
    }
}

/// Uma chamada no histórico.
#[derive(Debug, Clone, PartialEq)]
pub struct CallLog {
    pub id: String,
    pub peer: String,
    /// `in` ou `out`.
    pub dir: String,
    pub video: bool,
    pub ts: i64,
    /// Duração em segundos (0 se não foi atendida).
    pub dur: i64,
    /// `ok`, `missed`, `declined`, `busy`, `cancelled`, `failed`.
    pub status: String,
}

/// Um som criado pela pessoa para o soundboard.
#[derive(Debug, Clone, PartialEq)]
pub struct SoundRecord {
    pub id: String,
    pub name: String,
    pub emoji: String,
    pub dur: f64,
    pub mime: String,
    pub bytes: Vec<u8>,
    pub ts: i64,
}

/// Uma mensagem como ela fica guardada.
///
/// `body` carrega o JSON específico do tipo (texto, mídia, som, convite…), o
/// que permite guardar tipos novos sem mudar o formato da tabela.
#[derive(Debug, Clone, PartialEq)]
pub struct StoredMsg {
    pub id: String,
    pub chat: String,
    pub author: String,
    pub kind: String,
    pub ts: i64,
    pub body: String,
    pub status: Option<String>,
    pub exp: Option<i64>,
    pub reply: Option<String>,
    /// Reações em JSON: `{chave pública: emoji}`.
    pub reacts: String,
}

fn row_to_msg(r: &rusqlite::Row) -> rusqlite::Result<StoredMsg> {
    Ok(StoredMsg {
        id: r.get(0)?,
        chat: r.get(1)?,
        author: r.get(2)?,
        kind: r.get(3)?,
        ts: r.get(4)?,
        body: r.get(5)?,
        status: r.get(6)?,
        exp: r.get(7)?,
        reply: r.get(8)?,
        reacts: r.get(9)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(id: &str, chat: &str, ts: i64) -> StoredMsg {
        StoredMsg {
            id: id.into(),
            chat: chat.into(),
            author: "aa".into(),
            kind: "text".into(),
            ts,
            body: r#"{"text":"oi"}"#.into(),
            status: Some("sent".into()),
            exp: None,
            reply: None,
            reacts: "{}".into(),
        }
    }

    #[test]
    fn guarda_e_le_uma_mensagem() {
        let s = Store::in_memory().unwrap();
        assert!(s.put_msg(&msg("m1", "dm:bb", 100)).unwrap());
        assert_eq!(s.get_msg("m1").unwrap().unwrap().ts, 100);
    }

    #[test]
    fn a_mesma_mensagem_nao_entra_duas_vezes() {
        let s = Store::in_memory().unwrap();
        assert!(s.put_msg(&msg("m1", "dm:bb", 100)).unwrap());
        assert!(!s.put_msg(&msg("m1", "dm:bb", 100)).unwrap(), "veio por outro servidor");
        assert_eq!(s.count_msgs("dm:bb").unwrap(), 1);
    }

    #[test]
    fn envelope_ja_visto_e_reconhecido() {
        let s = Store::in_memory().unwrap();
        assert!(s.mark_seen("ev1", 10).unwrap(), "primeira vez");
        assert!(!s.mark_seen("ev1", 11).unwrap(), "chegou de novo por outro servidor");
    }

    #[test]
    fn pagina_do_mais_antigo_para_o_mais_novo() {
        let s = Store::in_memory().unwrap();
        for i in 0..10 {
            s.put_msg(&msg(&format!("m{i}"), "dm:bb", i * 10)).unwrap();
        }
        let p = s.msgs_by_chat("dm:bb", i64::MAX, 4).unwrap();
        assert_eq!(p.len(), 4);
        assert_eq!(p.first().unwrap().id, "m6", "pega as 4 mais recentes");
        assert_eq!(p.last().unwrap().id, "m9");
        assert!(p.windows(2).all(|w| w[0].ts <= w[1].ts), "em ordem de tempo");

        let anterior = s.msgs_by_chat("dm:bb", p.first().unwrap().ts, 3).unwrap();
        assert_eq!(anterior.last().unwrap().id, "m5", "rolar para trás continua de onde parou");
    }

    #[test]
    fn mensagens_temporarias_vencidas_sao_encontradas() {
        let s = Store::in_memory().unwrap();
        let mut a = msg("m1", "dm:bb", 100);
        a.exp = Some(50);
        let mut b = msg("m2", "dm:bb", 100);
        b.exp = Some(5000);
        s.put_msg(&a).unwrap();
        s.put_msg(&b).unwrap();
        s.put_msg(&msg("m3", "dm:bb", 100)).unwrap();

        let v = s.msgs_expired(1000).unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].id, "m1");
    }

    #[test]
    fn mídia_em_blocos_vira_arquivo_inteiro() {
        let s = Store::in_memory().unwrap();
        s.put_chunk("mid1", 1, "parte-b").unwrap();
        s.put_chunk("mid1", 0, "parte-a").unwrap();
        s.put_chunk("mid1", 1, "parte-b").unwrap(); // repetida
        let c = s.chunks_of("mid1").unwrap();
        assert_eq!(c, vec![(0, "parte-a".into()), (1, "parte-b".into())], "em ordem e sem repetir");

        s.put_media("mid1", "m1", "dm:bb", "image/webp", b"conteudo").unwrap();
        assert_eq!(s.media_bytes("mid1").unwrap().unwrap(), b"conteudo");
        assert!(s.chunks_of("mid1").unwrap().is_empty(), "blocos são liberados depois de montados");
        assert_eq!(s.media_size().unwrap(), 8);
    }

    #[test]
    fn apagar_conversa_leva_a_midia_junto() {
        let s = Store::in_memory().unwrap();
        s.put_msg(&msg("m1", "dm:bb", 1)).unwrap();
        s.put_media("mid1", "m1", "dm:bb", "image/webp", b"foto").unwrap();
        assert_eq!(s.delete_chat_msgs("dm:bb").unwrap(), 1);
        assert_eq!(s.count_msgs("dm:bb").unwrap(), 0);
        assert!(s.media_bytes("mid1").unwrap().is_none());
    }

    #[test]
    fn fila_de_envio_guarda_e_libera() {
        let s = Store::in_memory().unwrap();
        s.queue_outbox("m1", "[]", "[]", 5).unwrap();
        assert_eq!(s.outbox().unwrap().len(), 1);
        s.drop_outbox("m1").unwrap();
        assert!(s.outbox().unwrap().is_empty());
    }

    #[test]
    fn valores_soltos_sobrevivem() {
        let s = Store::in_memory().unwrap();
        s.set_kv("ajustes", &serde_json::json!({"tema": "escuro"})).unwrap();
        let v: serde_json::Value = s.get_kv("ajustes").unwrap().unwrap();
        assert_eq!(v["tema"], "escuro");
        assert!(s.get_kv::<serde_json::Value>("nao-existe").unwrap().is_none());
    }
}
