import { Metadata } from "./global.js"

/**
 * @param name - Name of the collection
 * @param uuid - UUID associated to the collection
 * @param collection - List of sub-collection within the collection
 * @param files - Map that link file within the collection name as key to the corresponding UUID.
 */
type Collection = {
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
    
    constructor(metadata:Map<string, Metadata>){
        this.metadata = metadata
        this.filesUUIDs = new Set();
        this.collectionUUIDs = new Set();

        const graphCollections: Map<string, string[]> = new Map()
        const graphFiles: Map<string, string[]> = new Map()
        graphCollections.set("", [])
        graphFiles.set("", [])
        this.metadata.forEach((data:Metadata, uuid:string) =>{
            switch (data.type) {
                case "DocumentType":
                    this.filesUUIDs.add(uuid)
                    break;
                case "CollectionType":
                    this.collectionUUIDs.add(uuid)
                    graphCollections.set(uuid, [])
                    graphFiles.set(uuid, [])
                    break
                default:
                    throw new Error(`Type ${data.type} is not supported.`)
            }
        });

        this.metadata.forEach((data:Metadata, uuid:string) =>{
            switch (data.type) {
                case "DocumentType":
                    graphFiles.get(data.parent)?.push(uuid)
                    break;
                case "CollectionType":
                    this.collectionUUIDs.add(uuid)
                    graphCollections.get(data.parent)?.push(uuid)
                    break
                default:
                    throw new Error(`Type ${data.type} is not supported.`)
            }
        });

        this.root = this.computeNode("", graphCollections, graphFiles)
    }

    private computeNode(
        uuid:string, 
        graphCollections:Map<string, string[]>, 
        graphFiles:Map<string, string[]>):Collection {

        let node:Collection = {
            name:this.metadata.get(uuid)?.visibleName ?? "",
            uuid:uuid, 
            collections:[], 
            files:new Map()
        }

        for (const uuidFile of graphFiles.get(uuid) ?? []){
            node.files.set(this.metadata.get(uuidFile)?.visibleName ?? "_error", uuidFile)
        }

        for (const uuidCollection of graphCollections.get(uuid) ?? []){
            node.collections.push(this.computeNode(uuidCollection, graphCollections, graphFiles))
        }

        return node
    }


}