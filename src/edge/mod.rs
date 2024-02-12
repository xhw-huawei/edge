mod inc;

use serde::Deserialize;
use std::io;

use crate::data::AsDataManager;

mod graph;

async fn invoke_inc(
    dm: &mut impl AsDataManager,
    root: &mut String,
    inc: &Inc,
) -> io::Result<InvokeResult> {
    match inc.code.as_str() {
        "return" => Ok(InvokeResult::Return(inc.target.clone())),
        "dump" => Ok(InvokeResult::Return(inc::dump(dm, &inc.target).await?)),
        "asign" => {
            inc::asign(dm, &root, &inc.source, &inc.target).await?;
            Ok(InvokeResult::Jump(1))
        }
        "delete" => {
            inc::delete(dm, &inc.target).await?;
            Ok(InvokeResult::Jump(1))
        }
        "dc" => {
            inc::delete_code(dm, &inc.target).await?;
            Ok(InvokeResult::Jump(1))
        }
        "dc_ns" => {
            let code = graph::get_target_anyway(dm, &inc.target, "$code").await?;
            let source_code = graph::get_target_anyway(dm, &inc.target, "$source_code").await?;
            inc::delete_code_without_source(dm, &code, &source_code).await?;
            Ok(InvokeResult::Jump(1))
        }
        "dc_nt" => {
            let code = graph::get_target_anyway(dm, &inc.target, "$code").await?;
            let target_code = graph::get_target_anyway(dm, &inc.target, "$target_code").await?;
            inc::delete_code_without_target(dm, &code, &target_code).await?;
            Ok(InvokeResult::Jump(1))
        }
        "set" => {
            inc::set(dm, &root, &inc.source, &inc.target).await?;
            Ok(InvokeResult::Jump(1))
        }
        "append" => {
            inc::append(dm, &root, &inc.source, &inc.target).await?;
            Ok(InvokeResult::Jump(1))
        }
        _ => todo!(),
    }
}

async fn unwrap_inc(dm: &mut impl AsDataManager, root: &str, inc: &Inc) -> io::Result<Inc> {
    Ok(Inc {
        source: inc::unwrap_value(dm, root, &inc.source).await?,
        code: inc::unwrap_value(dm, root, &inc.code).await?,
        target: inc::unwrap_value(dm, root, &inc.target).await?,
    })
}

// Public
#[derive(Clone, Deserialize)]
pub struct Inc {
    pub source: String,
    pub code: String,
    pub target: String,
}

pub enum InvokeResult {
    Jump(i32),
    Return(String),
}

pub trait AsEdgeEngine {
    async fn invoke_inc_v(&mut self, root: &mut String, inc_v: &Vec<Inc>) -> io::Result<String>;

    async fn commit(&mut self) -> io::Result<()>;
}

pub struct EdgeEngine<DM: AsDataManager> {
    dm: DM,
}

impl<DM: AsDataManager> EdgeEngine<DM> {
    pub fn new(dm: DM) -> Self {
        Self { dm }
    }
}

impl<DM: AsDataManager> AsEdgeEngine for EdgeEngine<DM> {
    async fn invoke_inc_v(&mut self, root: &mut String, inc_v: &Vec<Inc>) -> io::Result<String> {
        let mut pos = 0i32;
        let mut rs = String::new();
        while (pos as usize) < inc_v.len() {
            let inc = unwrap_inc(&mut self.dm, &root, &inc_v[pos as usize]).await?;
            match invoke_inc(&mut self.dm, root, &inc).await? {
                InvokeResult::Jump(step) => pos += step,
                InvokeResult::Return(s) => {
                    rs = s;
                    break;
                }
            }
        }
        Ok(rs)
    }

    async fn commit(&mut self) -> io::Result<()> {
        self.dm.commit().await
    }
}
