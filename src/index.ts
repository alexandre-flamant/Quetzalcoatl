import { ConnectOptions } from 'ssh2-sftp-client';
import { RM2Client } from './RM2Client.js';
import { FileSystem } from './FileSystem.js'

let client: RM2Client = new RM2Client()
let options: ConnectOptions = {
    host: '10.11.99.1',
    username:'root',
    password:'fZ5WkZ5VmC',
    port:22
}
await client.connect(options);

let uuids = await client.listUUID();
let metadata = await client.getMetadata(uuids)

let fileSystem = new FileSystem(metadata)

//client.disconnect();

console.log("done")
