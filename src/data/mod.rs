use std::io;

use sqlx::MySqlConnection;

use crate::mem_table::MemTable;

mod dao;

fn is_temp(source: &str, code: &str, target: &str) -> bool {
    source.starts_with('$') || code.starts_with('$') || target.starts_with('$')
}

async fn commit(dm: &mut DataManager<'_>) -> io::Result<()> {
    dao::insert_edge_mp(dm.conn, &dm.mem_table.take()).await
}

// Public
pub trait AsDataManager: Send {
    fn insert_edge(
        &mut self,
        source: &str,
        code: &str,
        no: u64,
        target: &str,
    ) -> impl std::future::Future<Output = io::Result<String>> + Send;

    fn set_target(
        &mut self,
        source: &str,
        code: &str,
        target: &str,
    ) -> impl std::future::Future<Output = io::Result<String>> + Send;

    fn append_target(
        &mut self,
        source: &str,
        code: &str,
        target: &str,
    ) -> impl std::future::Future<Output = io::Result<String>> + Send;

    fn get_target(
        &mut self,
        source: &str,
        code: &str,
    ) -> impl std::future::Future<Output = io::Result<String>> + Send;

    fn get_source(
        &mut self,
        code: &str,
        target: &str,
    ) -> impl std::future::Future<Output = io::Result<String>> + Send;

    async fn get_target_v(&mut self, source: &str, code: &str) -> io::Result<Vec<String>>;

    async fn get_list(
        &mut self,
        root: &str,
        dimension_v: &Vec<String>,
        attr_v: &Vec<String>,
    ) -> io::Result<json::Array>;

    async fn commit(&mut self) -> io::Result<()>;

    async fn delete(&mut self, point: &str) -> io::Result<()>;

    async fn delete_code(&mut self, code: &str) -> io::Result<()>;

    async fn delete_code_without_source(&mut self, code: &str, source_code: &str)
        -> io::Result<()>;

    async fn delete_code_without_target(&mut self, code: &str, target_code: &str)
        -> io::Result<()>;
}

pub struct DataManager<'a> {
    conn: &'a mut MySqlConnection,
    mem_table: &'a mut MemTable,
}

impl<'a> DataManager<'a> {
    pub fn new(conn: &'a mut MySqlConnection, mem_table: &'a mut MemTable) -> Self {
        Self { conn, mem_table }
    }
}

impl<'a> AsDataManager for DataManager<'a> {
    async fn insert_edge(
        &mut self,
        source: &str,
        code: &str,
        no: u64,
        target: &str,
    ) -> io::Result<String> {
        if is_temp(source, code, target) {
            Ok(self.mem_table.insert_temp_edge(source, code, no, target))
        } else {
            Ok(self.mem_table.insert_edge(source, code, no, target))
        }
    }

    async fn set_target(&mut self, source: &str, code: &str, target: &str) -> io::Result<String> {
        if let Some(id) = self.mem_table.set_target(source, code, target) {
            Ok(id)
        } else {
            if is_temp(source, code, target) {
                Ok(self.mem_table.insert_temp_edge(source, code, 0, target))
            } else {
                let (id, no) = dao::set_target(&mut self.conn, source, code, target).await?;
                self.mem_table
                    .append_exists_edge(&id, source, code, no, target);
                Ok(id)
            }
        }
    }

    async fn append_target(
        &mut self,
        source: &str,
        code: &str,
        target: &str,
    ) -> io::Result<String> {
        if let Some((no, _)) = self.mem_table.get_target(source, code) {
            if is_temp(source, code, target) {
                Ok(self
                    .mem_table
                    .insert_temp_edge(source, code, no + 1, target))
            } else {
                Ok(self.mem_table.insert_edge(source, code, no + 1, target))
            }
        } else {
            if is_temp(source, code, target) {
                Ok(self.mem_table.insert_temp_edge(source, code, 0, target))
            } else {
                let (id, no) = dao::append_target(&mut self.conn, source, code, target).await?;
                self.mem_table
                    .append_exists_edge(&id, source, code, no, target);
                Ok(id)
            }
        }
    }

    async fn get_target(&mut self, source: &str, code: &str) -> io::Result<String> {
        if let Some((_, target)) = self.mem_table.get_target(source, code) {
            return Ok(target);
        } else {
            let (id, no, target) = dao::get_target(&mut self.conn, source, code).await?;
            self.mem_table
                .append_exists_edge(&id, source, code, no, &target);
            Ok(target)
        }
    }

    async fn get_source(&mut self, code: &str, target: &str) -> io::Result<String> {
        if let Some(source) = self.mem_table.get_source(code, target) {
            return Ok(source);
        } else {
            let (id, no, source) = dao::get_source(&mut self.conn, code, target).await?;
            self.mem_table
                .append_exists_edge(&id, &source, code, no, target);
            Ok(source)
        }
    }

    async fn get_target_v(&mut self, source: &str, code: &str) -> io::Result<Vec<String>> {
        if is_temp(source, code, "") {
            Ok(self.mem_table.get_target_v_unchecked(source, code))
        } else {
            commit(self).await?;
            dao::get_target_v(&mut self.conn, source, code).await
        }
    }

    async fn get_list(
        &mut self,
        root: &str,
        dimension_v: &Vec<String>,
        attr_v: &Vec<String>,
    ) -> io::Result<json::Array> {
        commit(self).await?;
        dao::get_list(&mut self.conn, root, dimension_v, attr_v).await
    }

    async fn commit(&mut self) -> io::Result<()> {
        commit(self).await
    }

    async fn delete(&mut self, point: &str) -> io::Result<()> {
        commit(self).await?;
        dao::delete(self.conn, point).await
    }

    async fn delete_code(&mut self, code: &str) -> io::Result<()> {
        commit(self).await?;
        dao::delete_code(self.conn, code).await
    }

    async fn delete_code_without_source(
        &mut self,
        code: &str,
        source_code: &str,
    ) -> io::Result<()> {
        commit(self).await?;
        dao::delete_code_without_source(self.conn, code, source_code).await
    }

    async fn delete_code_without_target(
        &mut self,
        code: &str,
        target_code: &str,
    ) -> io::Result<()> {
        commit(self).await?;
        dao::delete_code_without_target(self.conn, code, target_code).await
    }
}
