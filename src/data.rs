use std::{future, io, pin::Pin};

use edge_lib::{data::AsDataManager, Path};
use sqlx::{MySql, Pool};

mod dao;

// Public
#[derive(Clone)]
pub struct DbDataManager {
    pool: Pool<MySql>,
}

impl DbDataManager {
    pub fn new(global: Pool<MySql>) -> Self {
        Self { pool: global }
    }
}

impl AsDataManager for DbDataManager {
    fn divide(&self) -> Box<dyn AsDataManager> {
        Box::new(Self {
            pool: self.pool.clone(),
        })
    }

    fn commit(&mut self) -> Pin<Box<dyn std::future::Future<Output = io::Result<()>> + Send>> {
        Box::pin(future::ready(Ok(())))
    }

    fn append(
        &mut self,
        path: &Path,
        item_v: Vec<String>,
    ) -> Pin<Box<dyn std::future::Future<Output = io::Result<()>> + Send>> {
        if path.step_v.is_empty() {
            return Box::pin(future::ready(Ok(())));
        }
        let mut this = self.clone();
        let mut path = path.clone();
        Box::pin(async move {
            let step = path.step_v.pop().unwrap();
            let root_v = this.get(&path).await?;
            for source in &root_v {
                dao::insert_edge(this.pool.clone(), source, &step.code, &item_v).await?;
            }
            Ok(())
        })
    }

    fn set(
        &mut self,
        path: &Path,
        item_v: Vec<String>,
    ) -> Pin<Box<dyn std::future::Future<Output = io::Result<()>> + Send>> {
        if path.step_v.is_empty() {
            return Box::pin(future::ready(Ok(())));
        }
        let mut this = self.clone();
        let mut path = path.clone();
        Box::pin(async move {
            let step = path.step_v.pop().unwrap();
            let root_v = this.get(&path).await?;
            for source in &root_v {
                dao::delete_edge_with_source_code(this.pool.clone(), source, &step.code).await?;
                dao::insert_edge(this.pool.clone(), source, &step.code, &item_v).await?;
            }
            Ok(())
        })
    }

    fn get(
        &mut self,
        path: &Path,
    ) -> Pin<Box<dyn std::future::Future<Output = io::Result<Vec<String>>> + Send>> {
        let this = self.clone();
        let mut path = path.clone();
        Box::pin(async move {
            let mut rs = vec![path.root.clone()];
            while !path.step_v.is_empty() {
                let step = path.step_v.remove(0);
                if step.arrow == "->" {
                    let mut n_rs = Vec::new();
                    for source in &rs {
                        n_rs.extend(
                            dao::get_target_v(this.pool.clone(), source, &step.code).await?,
                        );
                    }
                    rs = n_rs;
                } else {
                    let mut n_rs = Vec::new();
                    for target in &rs {
                        n_rs.extend(
                            dao::get_source_v(this.pool.clone(), &step.code, target).await?,
                        );
                    }
                    rs = n_rs;
                }
            }
            Ok(rs)
        })
    }
}
