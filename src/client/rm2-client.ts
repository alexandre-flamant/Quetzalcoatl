import { Client } from "ssh2";
import SFTPClient from "ssh2-sftp-client";
import { ConnectOptions } from "ssh2-sftp-client";
import { XOCHITL_PATH } from "../global.js";
import { Readable, Writable } from "stream";
import fs from "fs";
import path from "path";
import { RM2FileData, DefaultConnectOptions } from "./rm2-client.types.js";
import { Metadata } from "../filesystem/filesystem.types.js";

/**
 * This client manages the connection to the Remarkable 2 Xochilt file system through an SFTP session.
 * Its function is to:
 *  - Initialize and close the connection
 *  - List files and folders in the Xochitl file system
 *  - Download files
 *  - Upload files
 * @member client - sftp client to communicate with the tablet
 */
export class RM2Client {
  client: SFTPClient;
  connected: boolean;
  private connectOptions: ConnectOptions;

  /**
   * Initialize the client
   */
  constructor(options: ConnectOptions | DefaultConnectOptions) {
    this.client = new SFTPClient();
    this.connected = false;

    // Convert add default port if missing
    const connectOptions: ConnectOptions = {
      ...options,
      port: (options as ConnectOptions).port ?? 22,
    };

    this.connectOptions = connectOptions;
  }

  /**
   * Connect the client to the given tablet. Port SFTP port is 22 by default.
   *
   * @param options - Connection options
   */
  async connect() {
    // Initialize connection
    console.log(
      `Connecting to ${this.connectOptions.username}@${this.connectOptions.host}:${this.connectOptions.port}`
    );
    try {
      await this.client.connect(this.connectOptions);
      this.connected = true;
      console.log(
        `Connected to ${this.connectOptions.username}@${this.connectOptions.host}:${this.connectOptions.port}`
      );
    } catch (err) {
      console.log(`Failed to connect: ${err}`);
    }
  }

  /**
   * Reload the Xochitl process on the tablet. This is equivalent to reboot the tablet.
   * This is the only way I found for now to make the Remarkable 2 aware of the files.
   */
  async reloadXochitl() {
    // Setting client up
    const client = new Client();
    client
      .on("ready", () =>
        client.exec("systemctl restart xochitl", (err, _) => {
          if (err) console.log("Could not restart xochitl");
        })
      )
      .on("error", (error) => console.log("Error during Xochitl reloading."));

    // Connection
    client.connect({
      host: this.connectOptions.host,
      username: this.connectOptions.username,
      password: this.connectOptions.password,
    });
  }

  /**
   * Disconnect from the server.
   */
  async disconnect() {
    console.log(`Disconnecting.`);
    try {
      await this.client.end();
      console.log(`Disconnected.`);
    } catch (err) {
      console.log(`Failed to disconnect.`);
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
  async listUUID(): Promise<string[]> {
    let fileNames: string[] = [
      ...new Set(
        (await this.client.list(XOCHITL_PATH))
          .filter((info) => info.type != "d")
          .map((info) => info.name)
          .filter((info) => info.split(".")[1] != "tombstone")
          .filter((info) => info != ".tree")
          .map((str) => str.split(".")[0])
      ),
    ];

    return fileNames;
  }

  /**
   * Parse the metadata for all provided UUID.
   * @param uuids - UUIDs of the files to parse
   * @returns Promise of a Map<UUID, Metadata> that resolves when all metadata are parsed.
   */
  async getMetadata(uuids: string[]) {
    const metadataPromise = new Promise<Map<string, Metadata>>(
      (resolveMetadata, rejectMetadata) => {
        const promises: Promise<void>[] = []; // Promises of each stream
        let metadata: Map<string, Metadata> = new Map();

        for (const uuid of uuids) {
          // Create a promise for the stream
          const promise = new Promise<void>((resolve, reject) => {
            // Create the stream
            let dataTxt: string = "";
            const fileName: string = `${XOCHITL_PATH}/${uuid}.metadata`;
            const stream = this.client.createReadStream(fileName, {
              encoding: "utf-8",
            });

            // Handling stream
            stream.on("data", (b: Buffer) => {
              dataTxt += b;
            });
            stream.on("end", () => {
              try {
                const data: Metadata = JSON.parse(dataTxt);
                metadata.set(uuid, data);
                resolve(); // resolve the stream when it ends
              } catch (error) {
                reject();
              }
            });
            stream.on("error", (err: Error) =>
              console.log(`Error in ${uuid}.metadata: ${err.message}`)
            );
          });

          // Add this promise to the pool of stream promises
          promises.push(promise);
        }

        // Resolve the metadata promise when all stream are consumed.
        Promise.all(promises)
          .then(() => {
            resolveMetadata(metadata);
            console.log(`Number of UUID parsed: ${metadata.size}`);
          })
          .catch(rejectMetadata);
      }
    );
    return metadataPromise;
  }

  /**
   * Write the file and is data to the tablet using streams.
   *
   * @param uuid - Unique Universal Identifier of the file
   * @param filepath - Path of the file to be uploaded
   * @param data - Data generated from the file
   */
  async writeFiles(uuid: string, filepath: string, data: RM2FileData) {
    let writePromise: Promise<undefined> = new Promise(
      (resolveWrite, rejectWrite) => {
        const extension: string = path.extname(filepath);

        // Write the file
        let promiseFile = new Promise((resolve, reject) => {
          var readStream: Readable = fs.createReadStream(filepath);
          var writeStream: Writable = this.client.createWriteStream(
            `${XOCHITL_PATH}/${uuid}${extension}`
          );
          readStream.pipe(writeStream);
          writeStream.on("error", (err: Error) => {
            throw err;
          });
          writeStream.on("close", () => {
            resolve(undefined);
            console.log(`${uuid}.${extension} written`);
          });
        });

        // Write .pagedata file
        let promisePagedata = new Promise((resolve, reject) => {
          var readStream: Readable = Readable.from([data.pagedata]);
          var writeStream: Writable = this.client.createWriteStream(
            `${XOCHITL_PATH}/${uuid}.pagedata`
          );
          readStream.pipe(writeStream);
          writeStream.on("error", (err: Error) => {
            throw err;
          });
          writeStream.on("close", () => {
            resolve(undefined);
            new Promise(() => console.log(`${uuid}.pagedata written`));
          });
        });

        // Write .metadata file
        let promiseMetadata = new Promise((resolve, reject) => {
          var readStream: Readable = Readable.from([
            JSON.stringify(data.metadata, null, "\t"),
          ]);
          var writeStream: Writable = this.client.createWriteStream(
            `${XOCHITL_PATH}/${uuid}.metadata`
          );
          readStream.pipe(writeStream);
          writeStream.on("error", (err: Error) => {
            throw err;
          });
          writeStream.on("close", () => {
            resolve(undefined);
            console.log(`${uuid}.metadata written`);
          });
        });

        // Write .content file
        let promiseContent = new Promise((resolve, reject) => {
          var readStream: Readable = Readable.from([
            JSON.stringify(data.content, null, "\t"),
          ]);
          var writeStream: Writable = this.client.createWriteStream(
            `${XOCHITL_PATH}/${uuid}.content`
          );
          readStream.pipe(writeStream);
          writeStream.on("error", (err: Error) => {
            throw err;
          });
          writeStream.on("close", () => {
            resolve(undefined);
            console.log(`${uuid}.content written`);
          });
        });

        // Write .local file
        let promiseLocal = new Promise((resolve, reject) => {
          var readStream: Readable = Readable.from([
            JSON.stringify(data.local, null, "\t"),
          ]);
          var writeStream: Writable = this.client.createWriteStream(
            `${XOCHITL_PATH}/${uuid}.local`
          );
          readStream.pipe(writeStream);
          writeStream.on("error", (err: Error) => {
            throw err;
          });
          writeStream.on("close", () => {
            resolve(undefined);
            console.log(`${uuid}.local written`);
          });
        });
        // Create files
        let dir1Promise = this.client.mkdir(`${XOCHITL_PATH}/${uuid}`);
        let dir2Promise = this.client.mkdir(
          `${XOCHITL_PATH}/${uuid}.thumbnails`
        );
        Promise.all([promiseContent, promiseFile, promiseLocal, promiseMetadata, promisePagedata])
          .then(() => {
            resolveWrite(undefined);
            console.log(`Transfer of ${path.basename(filepath)} done under ${uuid}`);
          })
          .catch(rejectWrite);
      } 
    );
    
    return writePromise;
  }
}
