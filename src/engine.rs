use crate::index::Index;
use std::collections::HashMap;

pub struct Engine{
    index: Index
}

impl Engine{
    pub fn new(index: Index) -> Self{
        Self{
            index
        }
    }
    
    pub fn index(&mut self){
        println!("indexing");
        self.index.build_index();
    }

    pub fn search(&mut self, query: String){
        println!("Searching for '{}'", query);
        let mut relevant_docs: HashMap<String, f64> = HashMap::new();
        for term in query.split_whitespace(){
            let term_relevant_docs = self.index.find(term);
            term_relevant_docs.into_iter().for_each(|(doc, tfidf)|{
                if !relevant_docs.contains_key(&doc){
                    relevant_docs.insert(doc, tfidf);
                } else {
                    *relevant_docs.get_mut(&doc).unwrap() += tfidf;
                }
            });
        }
        let mut rows: Vec<_> = relevant_docs.into_iter().map(|(doc, tfidf)| (doc, tfidf)).collect();
        rows.sort_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap());
        for (doc, tfidf) in rows {
            println!("{} -> {}", doc, tfidf);
        }
    }
}