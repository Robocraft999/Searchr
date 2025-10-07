use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use rusqlite::{params, Connection};

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
            let mut inverted_index: HashMap<String, Vec<String>> = HashMap::new();
            let mut documents = Vec::new();

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

            for source in self.sources.iter() {
                let source_documents = source.resolve();
                for document in source_documents {
                    documents.push(document.name.clone());
                    conn.execute("INSERT INTO documents VALUES (?1, ?2)", (document.name.as_str(), "")).unwrap();

                    let mut words2 = HashMap::new();
                    for word in document.terms.iter() {
                        if !words2.contains_key(word) {
                            words2.insert(word.clone(), document.terms.iter().filter(|&w| w == word).count());
                        }
                    }

                    for (word, count) in words2 {
                        let tf = count as f64 / document.terms.len() as f64;

                        if !inverted_index.contains_key(&word) {
                            inverted_index.insert(word.clone(), Vec::new());
                            conn.execute("INSERT INTO terms VALUES (?1, ?2)", (word.as_str(), 0)).unwrap();
                        }
                        inverted_index.get_mut(&word).unwrap().push(document.name.clone());

                        conn.execute("INSERT INTO doc_terms VALUES (?1, ?2, ?3)", (document.name.as_str(), word.as_str(), tf)).unwrap();
                    }
                }
            }

            for (word, docs) in inverted_index{
                let idf = (documents.len() as f64 / docs.len() as f64).log10();

                conn.execute("INSERT OR REPLACE INTO terms VALUES (?1, ?2)", (word.as_str(), idf)).unwrap();
            }
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
            println!("{path:?}");
            let mut lines = String::new();
            let mut file = File::open(path.clone()).unwrap();
            let _ = file.read_to_string(&mut lines);
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