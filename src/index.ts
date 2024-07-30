import { ConnectOptions } from "ssh2-sftp-client";
import { RM2Client } from "./client/rm2-client.js";
import { RM2FileSystem } from "./filesystem/filesystem.js";

const filepath: string =
  "temp/One Piece T067 (Eiichiro ODA) [eBook officiel 1920].pdf";

let options: ConnectOptions = {
  host: "10.11.99.1",
  username: "root",
  password: "fZ5WkZ5VmC",
  port: 22,
};

let client: RM2Client = new RM2Client(options);
await client.connect();

let uuids = await client.listUUID();
//let metadata = await client.getMetadata(uuids)

//let fileSystem = new FileSystem(metadata)
let x = await RM2FileSystem.convert(filepath);
client.writeFiles("000000-0000-0000-0000-000000000001", filepath, x);

//client.reloadXochitl()

await new Promise((r) => setTimeout(r, 2000));
console.log("done");
//console.log(x);
