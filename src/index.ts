import { ConnectOptions } from 'ssh2-sftp-client';
import { SFTPClient } from './sftp.js';

let client: SFTPClient = new SFTPClient()
let options: ConnectOptions = {
    host: '10.11.99.1',
    username:'root',
    password:'fZ5WkZ5VmC',
    port:22
}
await client.connect(options);

let x = await client.listFiles();
x.forEach((x)=>console.log(x))

client.disconnect();