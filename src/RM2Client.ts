import Client, { FileInfo } from 'ssh2-sftp-client';
import { ConnectOptions } from 'ssh2-sftp-client';
import { DefaultConnectOptions, Metadata, XOCHITL_PATH } from './global.js';
import { Readable } from 'stream';

/**
 * This client manages the connection to the Remarkable 2 Xochilt file system through an SFTP session.
 * Its function is to:
 *  - Initialize and close the connection
 *  - List files and folders in the Xochitl file system
 *  - Download files
 *  - Upload files
 */
export class RM2Client {
    client: Client;
    connected:boolean;

    /**
     * Initialize the client
     */
    constructor() {
        this.client = new Client()
        this.connected = false;
    }

    /**
     * Connect the client to the given tablet. Port SFTP port is 22 by default.
     * 
     * @param options - Connection options
     */
    async connect(options: ConnectOptions|DefaultConnectOptions){
        // Convert add default port if missing
        const connectOptions: ConnectOptions = {
            ...options,
            port: (options as ConnectOptions).port ?? 22
        };
        
        // Initialize connection
        console.log(`Connecting to ${options.username}@${options.host}:${connectOptions.port}`)
        try{
            await this.client.connect(connectOptions)
            this.connected=true;
            console.log(`Connected to ${options.username}@${options.host}:${connectOptions.port}`)
        } catch (err){
            console.log(`Failed to connect: ${err}`)
        }
    }

    /**
     * Disconnect from the server.
     */
    async disconnect(){
        console.log(`Disconnecting.`)
        try{
            await this.client.end();
            console.log(`Disconnected.`)
        } catch (err){
            console.log(`Failed to disconnect.`)
        }
        this.connected = false;
    }

    /**
     * Return a list of all the UUIDs of all non deleted notebook in the 
     * xochitl File system.
     * See <https://remarkable.jms1.info/info/filesystem.html> for insights.
     * 
     * The following are filtered:
     *  - Directories
     *  - .tombstone files
     *  - .tree file
     * 
     * @returns List of all UUIDs
     */
    async listUUID():Promise<string[]>{
        let fileNames: string[] = [... new Set((await this.client.list(XOCHITL_PATH))
        .filter((info)=> info.type != 'd')
        .map((info) => info.name)
        .filter((info)=> info.split('.')[1] != 'tombstone')
        .filter((info)=> info != ".tree")
        .map((str) => str.split('.')[0])
        )]
        
        return fileNames;
    }

    /**
     * Parse the metadata for all provided UUID.
     * @param uuids - UUIDs of the files to parse
     * @returns Promise of a Map<UUID, Metadata> that resolves when all metadata are parsed.
     */
    async getMetadata(uuids:string[]){
        const metadataPromise = new Promise<Map<string, Metadata>>((resolveMetadata, rejectMetadata) => {
            const promises: Promise<void>[] = []; // Promises of each stream
            let metadata: Map<string, Metadata> = new Map();

            for(const uuid of uuids){
                // Create a promise for the stream
                const promise = new Promise<void>((resolve, reject) => {
                    // Create the stream
                    let dataTxt:string = "";
                    const fileName:string = `${XOCHITL_PATH}/${uuid}.metadata`;
                    const stream = this.client.createReadStream(fileName, {encoding:"utf-8"});
                    
                    // Handling stream
                    stream.on("data", (b:Buffer)=>{dataTxt += b;});
                    stream.on("end", ()=>{
                        try{
                            const data:Metadata = JSON.parse(dataTxt)
                            metadata.set(uuid, data)
                            resolve(); // resolve the stream when it ends
                        }catch(error){
                            reject();
                        }
                    });
                    stream.on("error", (err:Error)=>console.log(`Error in ${uuid}.metadata: ${err.message}`))
                });
                
                // Add this promise to the pool of stream promises
                promises.push(promise);
            }

            // Resolve the metadata promise when all stream are consumed.
            Promise.all(promises)
            .then(() => {
                resolveMetadata(metadata);
                console.log(`Number of UUID parsed: ${metadata.size}`)
            })
            .catch(rejectMetadata);
        });
        return metadataPromise
    }
}