// Constants
export const XOCHITL_PATH:string = "/home/root/.local/share/remarkable/xochitl"

// Types
/**
 * ConnectOption without the port as SFTP default port is 22
 */
export type DefaultConnectOptions = {
    host:string, 
    username:string, 
    password:string,
};

/**
 * Content of .metadata files
 */
export type Metadata = {
    createdTime: string,
    lastModified: string,
    lastOpened: string,
    lastOpenedPage: number,
    parent: string,
    pinned: boolean,
    type: string,
    visibleName: string
}
