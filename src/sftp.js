let Client = require("ssh2-sftp-client")
const { XOCHITL_PATH } = require("./tools")

class SFTPClient {
    /**
     * Create the SFTP client instance
     */
    constructor() {
        this.client = new Client()
    }

    /**
     * Connect the client to the given server. Port SFTP port is usually 22.
     * 
     * @param {{host:string, usernamestring, password:string, port:Number}} options - Connection options
     * 
     */
    async connect(options) {
        console.log(`Connecting to ${options.username}@${options.host}:${options.port}`)
        try{
            await this.client.connect(options)
            console.log(`Connected to ${options.username}@${options.host}:${options.port}`)
        } catch (err){
            console.log(`Failed to connect: ${err}`)
        }
    }

    /**
     * Disconnect from the server.
     */
    async disconnect(){
        console.log(`Disconnecting to ${options.username}@${options.host}:${options.port}`)
        try{
            await this.client.end();
            console.log(`Disconnected to ${options.username}@${options.host}:${options.port}`)
        } catch (err){
            console.log(`Failed to disconnect: ${err}`)
        }
    }

    /**
     * return a list of all the files in the xochitl file system of the remarkable tablet. 
     * @returns {Pro}
     */
    async listFiles(){
        return this.client.list(XOCHITL_PATH)
    }
}

module.exports = {SFTPClient}