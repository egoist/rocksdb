// @ts-check

import test from "ava"
import fs from "fs"

import { connect, getItem, setItem, getKeys, close } from "../index.js"

test.before("cleanup", async () => {
  await fs.promises.rm("test.db", { recursive: true, force: true })
})

test("basic", async (t) => {
  const db = await connect("test.db", {
    createIfMissing: true,
    keepLogFileNum: 10,
  })

  t.deepEqual(await getKeys(db), [])

  await setItem(db, "key1", "value1")

  t.deepEqual(await getKeys(db), ["key1"])

  t.is(await getItem(db, "key1"), "value1")

  await close(db)
})
