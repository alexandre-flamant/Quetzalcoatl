import { ConnectOptions } from 'ssh2-sftp-client';
import { RM2Client } from './RM2Client.js';
import { FileSystem } from './FileSystem.js'
import { convert } from './FileSystem.js';

const filepath: string = 'test/pdf/ref.pdf'

let options: ConnectOptions = {
    host: '10.11.99.1',
    username:'root',
    password:'fZ5WkZ5VmC',
    port:22
}

//let client: RM2Client = new RM2Client(options)
//await client.connect();

//let uuids = await client.listUUID();
//let metadata = await client.getMetadata(uuids)

//let fileSystem = new FileSystem(metadata)

//client.reloadXochitl()

//client.disconnect();

    
let x = await convert(filepath)
await new Promise(r => setTimeout(r, 2000));

console.log("done");