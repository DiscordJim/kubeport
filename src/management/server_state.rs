use std::{collections::HashSet, hash::Hash, sync::Arc};

use dashmap::DashSet;
use rand::{distributions::Standard, prelude::Distribution, Rng};
use tokio::sync::{Semaphore, SemaphorePermit};

#[derive(Default)]
pub struct IDIssuer<T>
where
    T: Copy + Eq + Hash,
    Standard: Distribution<T>
{
    ids: DashSet<T>
}

impl<T> IDIssuer<T>
where
    T: Copy + Eq + Hash,
    Standard: Distribution<T>
{
    pub fn issue(&self) -> IDPermit<T> {
        let mut thread_rng = rand::thread_rng();
        let mut gen: T = thread_rng.gen();
        
        while self.ids.contains(&gen) {
            gen = thread_rng.gen();
        }

        

        IDPermit::new(self, gen)
    }

    pub fn outstanding(&self, permit: &T) -> bool {
        self.ids.contains(permit)
    }

}

#[derive(Clone)]
pub struct IDPermit<'a, T>
where 
    T: Copy + Eq + Hash,
    Standard: Distribution<T>
{
    link: &'a IDIssuer<T>,
    permit: T
}


impl<'a, T> IDPermit<'a, T>
where 
    T: Copy + Eq + Hash,
    Standard: Distribution<T>
{

    fn new(link: &'a IDIssuer<T>, permit: T) -> Self {
        link.ids.insert(permit);
        Self {
            link,
            permit
        }
    }
    pub fn id(&self) -> T {
        self.permit
    }
}


impl<'a, T> Drop for IDPermit<'a, T>
where 
    T: Copy + Eq + Hash,
    Standard: Distribution<T>
{
    fn drop(&mut self) {
        self.link.ids.remove(&self.permit);
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::IDIssuer;

    #[test]
    pub fn issue() -> Result<()> {
        let issuer = IDIssuer::<u128>::default();
        let id = issuer.issue();
   

        let actual_id = id.id();

        assert!(issuer.outstanding(&actual_id));


        drop(id);

        assert!(!issuer.outstanding(&actual_id));







        Ok(())
    }
}