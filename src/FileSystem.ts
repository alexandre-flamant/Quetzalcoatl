import { Metadata } from "./global.js"

/**
 * @param name - Name of the collection
 * @param uuid - UUID associated to the collection
 * @param collection - List of sub-collection within the collection
 * @param files - Map that link file within the collection name as key to the corresponding UUID.
 */
export type Collection = {
    name:string,
    uuid:string,
    collections:Collection[]
    files:Map<string, string>
}

export class FileSystem {
    root:Collection
    filesUUIDs: Set<string>
    collectionUUIDs: Set<string>
    metadata: Map<string, Metadata>

    private graphCollections: Map<string, string[]>
    private graphFiles: Map<string, string[]>

    constructor(metadata:Map<string, Metadata>){
        this.metadata = metadata
        this.filesUUIDs = new Set();
        this.collectionUUIDs = new Set();
        
        // Initialize the directed acyclic graph
        this.graphCollections= new Map()
        this.graphFiles = new Map()

        this.graphCollections.set("", [])
        this.graphFiles.set("", [])
        this.metadata.forEach((data:Metadata, uuid:string) =>{
            switch (data.type) {
                case "DocumentType":
                    this.filesUUIDs.add(uuid)
                    break;
                case "CollectionType":
                    this.collectionUUIDs.add(uuid)
                    this.graphCollections.set(uuid, [])
                    this.graphFiles.set(uuid, [])
                    break
                default:
                    throw new Error(`Type ${data.type} is not supported.`)
            }
        });
        
        // Fill the graph
        this.metadata.forEach((data:Metadata, uuid:string) =>{
            switch (data.type) {
                case "DocumentType":
                    this.graphFiles.get(data.parent)?.push(uuid)
                    break;
                case "CollectionType":
                    this.collectionUUIDs.add(uuid)
                    this.graphCollections.get(data.parent)?.push(uuid)
                    break
                default:
                    throw new Error(`Type ${data.type} is not supported.`)
            }
        });

        // Compute the root of the file system representation
        this.root = this.computeNode("")
    }

    /**
     * Compute the graph node of a Collection using recursive exploration of the graph from Xochitl file system.
     * @param uuid - UUID of the node to generate the representation of
     * @returns The representation of the node as a Collection.
     */
    private computeNode(uuid:string):Collection {

        let node:Collection = {
            name:this.metadata.get(uuid)?.visibleName ?? "",
            uuid:uuid, 
            collections:[], 
            files:new Map()
        }

        this.graphFiles.get(uuid)?.forEach((fileUUID)=>{
            const fileName:string|undefined = this.metadata.get(fileUUID)?.visibleName
            if (fileName) node.files.set(fileName , fileUUID)
        })

        this.graphCollections.get(uuid)?.forEach((collectionUUID)=>
            node.collections.push(this.computeNode(collectionUUID))
        )

        return node
    }


}