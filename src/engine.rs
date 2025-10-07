use crate::index::Index;

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
}