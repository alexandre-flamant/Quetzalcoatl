import { Content, Metadata } from "../filesystem/filesystem.types.js";

// Types

/**
 * ConnectOption without the port as SFTP default port is 22
 * @member host - Host to connect to
 * @member username - Username to connect with
 * @member password - Password to connect with
 */
export type DefaultConnectOptions = {
  host: string;
  username: string;
  password: string;
};

/**
 * Data associated with file in Xochitl file system
 * @member metadata - Content of the .metadata file
 * @member content - Content of the .content file
 * @member local - Content of the .content file
 * @member pagedata - Content of the .pagedata file
 */
export type RM2FileData = {
  metadata: Metadata;
  content: Content;
  local: {
    contentFormatVersion: number;
  };
  pagedata: string;
};
