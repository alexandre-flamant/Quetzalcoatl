import { ConnectOptions } from "ssh2-sftp-client";
import { RM2Client } from "./client/rm2-client.js";
import { RM2FileSystem } from "./filesystem/filesystem.js";

const filepath: string = './test/pdf/ref.pdf'

let options: ConnectOptions = {
  host: "10.11.99.1",
  username: "root",
  password: "fZ5WkZ5VmC",
  port: 22,
};

let client: RM2Client = new RM2Client(options);
await client.connect();
let uuids = await client.listUUID()
let metadata = await client.getMetadata(uuids)
let fs = new RM2FileSystem(metadata)
let dirUUID = fs.getCollectionUUID("Livres//mangas//HXH")

let fileData = await RM2FileSystem.convert(filepath, dirUUID);
await client.writeFiles("000000-0000-0000-0000-000000000000", filepath, fileData);

client.reloadXochitl()