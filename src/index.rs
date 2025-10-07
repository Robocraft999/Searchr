use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use sqlite::{BindableWithIndex, Connection};

pub trait IndexSource{
    //TODO query type and type for results
    fn resolve(&self) -> Vec<(String, String)>;
}

pub struct Index{
    sources: Vec<Box<dyn IndexSource>>,
    connection: Option<Connection>,
}

impl Index{
    pub fn new() -> Self{
        let connection = sqlite::open("./db.sqlite").ok();
        Self{
            sources: Vec::new(),
            connection,
        }
    }

    pub fn build_index(&mut self){
        if let Some(conn) = &mut self.connection {
            let mut inverted_index: HashMap<String, Vec<String>> = HashMap::new();
            let mut documents = Vec::new();

            conn.execute("CREATE TABLE tf (token TEXT, val FLOAT);").expect("Could not create table");
            conn.execute("CREATE TABLE idf (token TEXT, val FLOAT);").expect("Could not create table");
            for source in self.sources.iter() {
                let pair = source.resolve();
                for (name, lines) in pair {
                    documents.push(name.clone());

                    let mut words2 = HashMap::new();
                    let words = lines.split_whitespace().map(|w| w.to_owned()).collect::<Vec<String>>();
                    for word in words.iter() {
                        if !words2.contains_key(word) {
                            words2.insert(word.clone(), words.iter().filter(|&w| w == word).count());
                        }
                    }

                    for (word, count) in words2 {
                        let tf = count as f64 / words.len() as f64;

                        if !inverted_index.contains_key(&word) {
                            inverted_index.insert(word.clone(), Vec::new());
                        }
                        inverted_index.get_mut(&word).unwrap().push(name.clone());

                        let query = "INSERT INTO tf VALUES (?, ?)";
                        let mut statement = conn.prepare(query).unwrap();
                        statement.bind((1, word.as_str())).unwrap();
                        statement.bind((2, tf)).unwrap();
                        let _ = statement.iter().last();
                    }
                }
            }

            for (word, docs) in inverted_index{
                let idf = (documents.len() as f64 / docs.len() as f64).log10();

                let query = "INSERT INTO idf VALUES (?, ?)";
                let mut statement = conn.prepare(query).unwrap();
                statement.bind((1, word.as_str())).unwrap();
                statement.bind((2, idf)).unwrap();
                let _ = statement.iter().last();
            }
        }
    }

    pub fn find(&self, term: String) -> Option<String>{
        if let Some(conn) = &self.connection {

            None
        } else {
            None
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
    fn resolve(&self) -> Vec<(String, String)> {
        let mut contents = Vec::new();
        for path in fs::read_dir(&self.path).unwrap().filter_map(|entry| entry.ok()).filter_map(|entry| if entry.path().is_file() {Some(entry.path())} else {None}){
            println!("{path:?}");
            let mut lines = String::new();
            let mut file = File::open(path.clone()).unwrap();
            let _ = file.read_to_string(&mut lines);
            contents.push((path.into_os_string().into_string().unwrap(), lines));
        }
        contents
    }
}