mod raw {
    use std::io::{self, Error, ErrorKind};

    use sqlx::{MySqlConnection, Row};

    pub async fn get_source(
        conn: &mut MySqlConnection,
        code: &str,
        target: &str,
    ) -> io::Result<String> {
        let row = sqlx::query("select source from edge_t where code=? and target=?")
            .bind(code)
            .bind(target)
            .fetch_one(conn)
            .await
            .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;
        Ok(row.get(0))
    }

    pub async fn delete_code(
        conn: &mut MySqlConnection,
        source: &str,
        code: &str,
    ) -> io::Result<()> {
        log::debug!("delete_code: {source}->{code}");

        sqlx::query("delete from edge_t where source=? and code=?")
            .bind(&source)
            .bind(&code)
            .execute(conn)
            .await
            .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;
        Ok(())
    }
}

use std::io::{self, Error, ErrorKind};

use sqlx::{MySqlConnection, Row};

#[async_recursion::async_recursion]
pub async fn get(conn: &mut MySqlConnection, root: &str, path: &str) -> io::Result<String> {
    let arrow = &path[0..2];
    let path = &path[2..];

    let _v = path.find("->");
    let v_ = path.find("<-");
    if _v.is_some() || v_.is_some() {
        let pos = if _v.is_some() && v_.is_some() {
            std::cmp::min(_v.unwrap(), v_.unwrap())
        } else if _v.is_some() {
            _v.unwrap()
        } else {
            v_.unwrap()
        };
        let code = &path[0..pos];
        let path = &path[pos + 2..];

        let pt = if arrow == "->" {
            get_target_anyway(conn, root, code).await?
        } else {
            get_source_anyway(conn, code, root).await?
        };
        get(conn, &pt, path).await
    } else {
        if arrow == "->" {
            get_target_anyway(conn, root, path).await
        } else {
            get_source_anyway(conn, path, root).await
        }
    }
}

pub fn new_point() -> String {
    uuid::Uuid::new_v4().to_string()
}

pub async fn insert_edge(
    conn: &mut MySqlConnection,
    source: &str,
    code: &str,
    target: &str,
) -> io::Result<String> {
    log::debug!("insert_edge: {source}->{code}={target}");

    let id = new_point();
    sqlx::query("insert into edge_t (id,source,code,target) values (?,?,?,?)")
        .bind(&id)
        .bind(&source)
        .bind(&code)
        .bind(&target)
        .execute(conn)
        .await
        .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;
    Ok(id)
}

pub async fn get_target(
    conn: &mut MySqlConnection,
    source: &str,
    code: &str,
) -> io::Result<String> {
    log::debug!("get_target: {source}->{code}=?");

    let row = sqlx::query("select target from edge_t where source=? and code=?")
        .bind(source)
        .bind(code)
        .fetch_one(conn)
        .await
        .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;
    let target = row.get(0);
    log::debug!("get_target: {source}->{code}={target}");

    Ok(target)
}

pub async fn get_target_anyway(
    conn: &mut MySqlConnection,
    source: &str,
    code: &str,
) -> io::Result<String> {
    match get_target(conn, source, code).await {
        Ok(target) => Ok(target),
        Err(_) => {
            let target = new_point();
            insert_edge(conn, source, code, &target).await?;
            Ok(target)
        }
    }
}

pub async fn get_source_anyway(
    conn: &mut MySqlConnection,
    code: &str,
    target: &str,
) -> io::Result<String> {
    match raw::get_source(conn, code, target).await {
        Ok(source) => Ok(source),
        Err(_) => {
            let source = new_point();
            insert_edge(conn, &source, code, target).await?;
            Ok(source)
        }
    }
}

pub async fn set_target(
    conn: &mut MySqlConnection,
    source: &str,
    code: &str,
    target: &str,
) -> io::Result<String> {
    raw::delete_code(conn, source, code).await?;
    insert_edge(conn, source, code, target).await
}
