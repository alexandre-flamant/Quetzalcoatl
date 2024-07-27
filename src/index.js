const { SFTPClient } = require( "./sftp.js" )
const sleep = (delay) => new Promise((resolve) => setTimeout(resolve, delay))

async function test(){
    client = new SFTPClient()
    options = {
        host: '10.11.99.1',
        username:'root',
        password:'fZ5WkZ5VmC',
        port:22
    }
    await client.connect(options)
    await sleep(5000)
    client.disconnect()
}

client = new SFTPClient()
options = {
    host: '10.11.99.1',
    username:'root',
    password:'fZ5WkZ5VmC',
    port:22
}
await client.connect(options)
list = client.listFiles()