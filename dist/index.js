import { SFTPClient } from './sftp.js';
let client = new SFTPClient();
let options = {
    host: '10.11.99.1',
    username: 'root',
    password: 'fZ5WkZ5VmC',
    port: 22
};
await client.connect(options);
let x = await client.listFiles();
x.forEach((x) => console.log(x));
client.disconnect();
//# sourceMappingURL=index.js.map