import Client from 'ssh2-sftp-client';
import { XOCHITL_PATH } from './global.js';
export class SFTPClient {
    client;
    connected;
    /**
     * Create the SFTP client instance
     */
    constructor() {
        this.client = new Client();
        this.connected = false;
    }
    /**
     * Connect the client to the given server. Port SFTP port is usually 22.
     *
     * @param options - Connection options
     */
    async connect(options) {
        console.log(`Connecting to ${options.username}@${options.host}:${options.port}`);
        try {
            await this.client.connect(options);
            this.connected = true;
            console.log(`Connected to ${options.username}@${options.host}:${options.port}`);
        }
        catch (err) {
            console.log(`Failed to connect: ${err}`);
        }
    }
    /**
     * Disconnect from the server.
     */
    async disconnect() {
        console.log(`Disconnecting.`);
        try {
            await this.client.end();
            console.log(`Disconnected.`);
        }
        catch (err) {
            console.log(`Failed to disconnect.`);
        }
        this.connected = false;
    }
    /**
     * Return a list of all the files in the xochitl file system of the remarkable tablet.
     * Directories are excluded.
     * @returns {Pro}
     */
    async listFiles() {
        let fileNames = [...new Set((await this.client.list(XOCHITL_PATH))
                .filter((info) => info.type != 'd')
                .map((info) => info.name)
                .filter((info) => info.split('.')[1] != 'tombstone')
                .filter((info) => info != ".tree")
                .map((str) => str.split('.')[0]))];
        return fileNames;
    }
}
//# sourceMappingURL=sftp.js.map