#![deny(clippy::all)]

use once_cell::sync::Lazy;
use std::sync::atomic::Ordering;
use std::sync::Mutex;
use std::{collections::HashMap, sync::atomic::AtomicU32};

use napi::{
  bindgen_prelude::{AbortSignal, AsyncTask},
  Task,
};

#[macro_use]
extern crate napi_derive;

pub struct Database {
  db: rocksdb::DB,
  db_opts: rocksdb::Options,
  filepath: String,
}

#[napi(object)]
pub struct Options {
  pub create_if_missing: bool,
  pub keep_log_file_num: u32,
}

static DB_ID: AtomicU32 = AtomicU32::new(0);

static DATABASE_INSTANCES: Lazy<Mutex<HashMap<u32, Database>>> =
  Lazy::new(|| Mutex::new(HashMap::new()));

pub struct ConnectTask {
  path: String,
  opts: Options,
}

#[napi]
impl Task for ConnectTask {
  type Output = u32;
  type JsValue = u32;

  fn compute(&mut self) -> napi::Result<Self::Output> {
    let mut db_opts = rocksdb::Options::default();
    db_opts.create_if_missing(self.opts.create_if_missing);
    db_opts.set_keep_log_file_num(self.opts.keep_log_file_num.try_into().unwrap());

    let db = rocksdb::DB::open(&db_opts, &self.path).unwrap();
    let db_instance = Database {
      db,
      db_opts,
      filepath: self.path.clone(),
    };

    let db_id = DB_ID.fetch_add(1, Ordering::Relaxed);
    let mut dbs = DATABASE_INSTANCES.lock().unwrap();
    dbs.insert(db_id, db_instance);
    Ok(db_id)
  }

  fn resolve(&mut self, _env: napi::Env, output: Self::Output) -> napi::Result<Self::JsValue> {
    Ok(output)
  }
}

#[napi]
pub fn connect(
  path: String,
  opts: Options,
  abort_signal: Option<AbortSignal>,
) -> AsyncTask<ConnectTask> {
  AsyncTask::with_optional_signal(ConnectTask { path, opts }, abort_signal)
}

pub struct GetItemTask {
  db_id: u32,
  key: String,
}

#[napi]
impl Task for GetItemTask {
  type Output = Option<String>;
  type JsValue = Option<String>;

  fn compute(&mut self) -> napi::Result<Self::Output> {
    let dbs = DATABASE_INSTANCES.lock().unwrap();
    let db = dbs.get(&self.db_id).unwrap();

    match db.db.get(&self.key) {
      Ok(Some(value)) => Ok(Some(String::from_utf8(value).unwrap())),
      Ok(None) => Ok(None),
      Err(e) => Err(napi::Error::new(
        napi::Status::GenericFailure,
        format!("{}", e),
      )),
    }
  }

  fn resolve(&mut self, _env: napi::Env, output: Self::Output) -> napi::Result<Self::JsValue> {
    Ok(output)
  }
}

#[napi]
pub fn get_item(
  db_id: u32,
  key: String,
  abort_signal: Option<AbortSignal>,
) -> AsyncTask<GetItemTask> {
  AsyncTask::with_optional_signal(GetItemTask { db_id, key }, abort_signal)
}

pub struct SetItemTask {
  db_id: u32,
  key: String,
  value: String,
}

#[napi]
impl Task for SetItemTask {
  type Output = ();
  type JsValue = ();

  fn compute(&mut self) -> napi::Result<Self::Output> {
    let dbs = DATABASE_INSTANCES.lock().unwrap();
    let db = dbs.get(&self.db_id).unwrap();

    db.db.put(&self.key, &self.value).unwrap();
    Ok(())
  }

  fn resolve(&mut self, _env: napi::Env, output: Self::Output) -> napi::Result<Self::JsValue> {
    Ok(output)
  }
}

#[napi]
pub fn set_item(
  db_id: u32,
  key: String,
  value: String,
  abort_signal: Option<AbortSignal>,
) -> AsyncTask<SetItemTask> {
  AsyncTask::with_optional_signal(SetItemTask { db_id, key, value }, abort_signal)
}

pub struct GetKeysTask {
  db_id: u32,
  prefix: Option<String>,
}

#[napi]
impl Task for GetKeysTask {
  type Output = Vec<String>;
  type JsValue = Vec<String>;

  fn compute(&mut self) -> napi::Result<Self::Output> {
    let dbs = DATABASE_INSTANCES.lock().unwrap();
    let db = dbs.get(&self.db_id).unwrap();

    let iter = db.db.iterator(rocksdb::IteratorMode::Start);
    let mut keys: Vec<String> = vec![];

    for item in iter {
      match item {
        Ok((key, _)) => {
          if let Some(prefix) = &self.prefix {
            if !key.starts_with(prefix.as_bytes()) {
              continue;
            }
          }

          keys.push(key.to_vec().into_iter().map(|c| c as char).collect());
        }
        Err(e) => {
          return Err(napi::Error::new(
            napi::Status::GenericFailure,
            format!("{}", e),
          ));
        }
      }
    }

    Ok(keys)
  }

  fn resolve(&mut self, _env: napi::Env, output: Self::Output) -> napi::Result<Self::JsValue> {
    Ok(output)
  }
}

#[napi]
pub fn get_keys(
  db_id: u32,
  prefix: Option<String>,
  abort_signal: Option<AbortSignal>,
) -> AsyncTask<GetKeysTask> {
  AsyncTask::with_optional_signal(GetKeysTask { db_id, prefix }, abort_signal)
}

pub struct RemoveItemTask {
  db_id: u32,
  key: String,
}

#[napi]
impl Task for RemoveItemTask {
  type Output = ();
  type JsValue = ();

  fn compute(&mut self) -> napi::Result<Self::Output> {
    let dbs = DATABASE_INSTANCES.lock().unwrap();
    let db = dbs.get(&self.db_id).unwrap();

    db.db.delete(&self.key).unwrap();
    Ok(())
  }

  fn resolve(&mut self, _env: napi::Env, output: Self::Output) -> napi::Result<Self::JsValue> {
    Ok(output)
  }
}

#[napi]
pub fn remove_item(
  db_id: u32,
  key: String,
  abort_signal: Option<AbortSignal>,
) -> AsyncTask<RemoveItemTask> {
  AsyncTask::with_optional_signal(RemoveItemTask { db_id, key }, abort_signal)
}

pub struct CloseTask {
  db_id: u32,
}

#[napi]
impl Task for CloseTask {
  type Output = ();
  type JsValue = ();

  fn compute(&mut self) -> napi::Result<Self::Output> {
    let mut dbs = DATABASE_INSTANCES.lock().unwrap();
    let db = dbs.get(&self.db_id).unwrap();
    println!("Destroying db: {}", &db.filepath);
    let _ = rocksdb::DB::destroy(&db.db_opts, &db.filepath);
    dbs.remove(&self.db_id);
    Ok(())
  }

  fn resolve(&mut self, _env: napi::Env, output: Self::Output) -> napi::Result<Self::JsValue> {
    Ok(output)
  }
}

#[napi]
pub fn close(db_id: u32, abort_signal: Option<AbortSignal>) -> AsyncTask<CloseTask> {
  AsyncTask::with_optional_signal(CloseTask { db_id }, abort_signal)
}
