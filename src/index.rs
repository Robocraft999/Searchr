use std::collections::HashMap;
use std::{fs, mem};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use rusqlite::{params, Connection, PrepFlags, TransactionBehavior};

pub trait IndexSource{
    //TODO query type and type for results
    fn resolve(&self) -> Vec<Document>;
}

pub type Term = String;
#[derive(Debug, PartialEq, Eq)]
pub struct Document{
    name: String,
    terms: Vec<Term>
}
struct FindRow(String, f64);

pub struct Index{
    sources: Vec<Box<dyn IndexSource>>,
    connection: Option<Connection>,
}

impl Index{
    pub fn new() -> Self{
        let connection = Connection::open("./db.sqlite").ok();
        Self{
            sources: Vec::new(),
            connection,
        }
    }

    pub fn build_index(&mut self){
        if let Some(conn) = &mut self.connection {
            let mut inverted_index: HashMap<String, usize> = HashMap::new();
            let mut documents = Vec::new();

            conn.pragma_update(None, "journal_mode", &"OFF").unwrap();

            conn.execute_batch("
                DROP TABLE IF EXISTS doc_terms;
                DROP TABLE IF EXISTS documents;
                DROP TABLE IF EXISTS terms;
                CREATE TABLE documents (
                    title       TEXT PRIMARY KEY,
                    content     TEXT
                );
                CREATE TABLE terms (
                    term        TEXT PRIMARY KEY,
                    idf         REAL NOT NULL
                );
                CREATE TABLE doc_terms (
                    doc_id      TEXT NOT NULL,
                    term_id     TEXT NOT NULL,
                    tf          REAL NOT NULL,
                    PRIMARY KEY (doc_id, term_id),
                    FOREIGN KEY (doc_id) REFERENCES documents(title),
                    FOREIGN KEY (term_id) REFERENCES terms(term)
                );
            "
            ).expect("Database setup failed");

            let transaction = conn.transaction_with_behavior(TransactionBehavior::Exclusive).unwrap();
            let mut document_stmt = transaction.prepare_with_flags("INSERT INTO documents VALUES (?1, ?2)", PrepFlags::SQLITE_PREPARE_PERSISTENT).unwrap();
            let mut term_stmt = transaction.prepare_with_flags("INSERT OR REPLACE INTO terms VALUES (?1, ?2)", PrepFlags::SQLITE_PREPARE_PERSISTENT).unwrap();
            let mut doc_term_stmt = transaction.prepare_with_flags("INSERT INTO doc_terms VALUES (?1, ?2, ?3)", PrepFlags::SQLITE_PREPARE_PERSISTENT).unwrap();


            for source in self.sources.iter() {
                let source_documents = source.resolve();
                for document in source_documents {
                    println!("{}", document.name);
                    documents.push(document.name.clone());
                    document_stmt.execute((document.name.as_str(), "")).unwrap();

                    let mut unique_term_counts = HashMap::new();
                    for word in document.terms.iter() {
                        if !unique_term_counts.contains_key(word) {
                            unique_term_counts.insert(word.clone(), 0);
                        }
                        *unique_term_counts.get_mut(&word.clone()).unwrap() += 1;
                    }

                    for (word, count) in unique_term_counts {
                        let tf = count as f64 / document.terms.len() as f64;

                        if !inverted_index.contains_key(&word) {
                            inverted_index.insert(word.clone(), 0);
                            term_stmt.execute((word.as_str(), 0)).unwrap();
                        }
                        *inverted_index.get_mut(&word).unwrap() += 1;

                        doc_term_stmt.execute((document.name.as_str(), word.as_str(), tf)).unwrap();
                    }
                }
            }

            for (term, doc_count) in inverted_index{
                let idf = (documents.len() as f64 / doc_count as f64).log10();

                term_stmt.execute((term.as_str(), idf)).unwrap();
            }
            document_stmt.finalize().unwrap();
            term_stmt.finalize().unwrap();
            doc_term_stmt.finalize().unwrap();
            transaction.commit().unwrap();
        }
    }

    pub fn find(&self, term: &str) -> HashMap<String, f64>{
        if let Some(conn) = &self.connection {
            let mut stmt = conn.prepare("
            SELECT
                d.title,
                dt.tf * t.idf AS tfidf
            FROM documents d
            JOIN doc_terms dt ON d.title = dt.doc_id
            JOIN terms t ON dt.term_id = t.term
            WHERE t.term LIKE ?1
            ").unwrap();
            stmt.query_map(params![term], |row| {
                Ok(FindRow(row.get(0)?, row.get(1)?))
            }).unwrap().map(|r| r.unwrap()).map(|fr| (fr.0, fr.1)).collect::<HashMap<String, f64>>()
        } else {
            HashMap::new()
        }
    }

    pub fn add(&mut self, source: Box<dyn IndexSource>){
        self.sources.push(source);
    }
}

pub struct LocalFilesystemSource {
    path: PathBuf
}

impl LocalFilesystemSource {
    pub fn new<P: AsRef<Path>>(path: P) -> Self{
        Self{
            path: path.as_ref().to_path_buf()
        }
    }
}

impl IndexSource for LocalFilesystemSource {
    fn resolve(&self) -> Vec<Document> {
        let mut contents = Vec::new();
        for path in fs::read_dir(&self.path).unwrap().filter_map(|entry| entry.ok()).filter_map(|entry| if entry.path().is_file() {Some(entry.path())} else {None}){
            let mut lines = String::new();
            let mut file = File::open(path.clone()).unwrap();
            match path.extension().unwrap().to_str().unwrap() {
                "html" => {
                    lines = html2text::config::plain().string_from_read(file, 50).unwrap();
                }
                _ => {
                    let _ = file.read_to_string(&mut lines);
                }
            }

            //TODO do proper term splitting
            let terms = lines.split_whitespace().map(|w| w.to_owned()).collect::<Vec<String>>();
            contents.push(Document{
                name: path.into_os_string().into_string().unwrap(),
                terms,
            });
        }
        contents
    }
}