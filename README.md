# @egoist/rocksdb

A simple crate to use RocksDB in Node.js

## Install

```bash
npm i @egoist/rocksdb
```

## Usage

```ts
import {
  connect,
  setItem,
  getItem,
  getKeys,
  removeItem,
  close,
} from "@egoist/rocksdb"

async function main() {
  const db = await connect("path/to/db")
  await setItem(db, "key", "value")
  console.log(await getItem(db, "key")) // value
  console.log(await getKeys(db)) // ['key']
  await removeItem(db, "key")
  console.log(await getItem(db, "key")) // null
  await close(db)
}
```

## License

MIT
