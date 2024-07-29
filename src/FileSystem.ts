import fs from 'fs'
import path from 'path';
import { PDFDocument } from 'pdf-lib'
import { v4 } from 'uuid'

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

export type Content = {
    coverPageNumber: number,
    customZoomCenterX: number,
    customZoomCenterY: number,
    customZoomOrientation: "portrait"|"landscape",
    customZoomPageHeight: number,
    customZoomPageWidth: number,
    customZoomScale: number,
    documentMetadata: {authors: string[]}, //
    extraMetadata: {},
    fileType: string,
    fontName: string,
    formatVersion: number,
    lineHeight: number,
    margins: number,
    orientation: "portrait"|"landscape",
    originalPageCount: number,   //
    pageCount: number,           //
    pageTags: [],
    pages: string[],             //
    redirectionPageMap: number[] //
    sizeInBytes: number,         //
    tags: [],
    textAlignment: "justify"|string,
    textScale: number,
    zoomMode: string
}

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

    static async convert(filepath:string){
        const metadata: Metadata = {} as Metadata
        await fs.promises.stat(filepath).then((stats)=>{
                metadata.createdTime = Math.floor(stats.ctimeMs).toString()
                metadata.lastModified = Math.floor(stats.mtimeMs).toString()
                metadata.lastOpened = "0"
                metadata.lastOpenedPage = 0
                metadata.parent = ""
                metadata.pinned = false
                metadata.type = "DocumentType"
                metadata.visibleName = "" 
            })
            
        metadata.visibleName = path.basename(filepath) 
        
        const pdfBuffer = fs.readFileSync(filepath);
        const pdfDoc = await PDFDocument.load(pdfBuffer);
        const pageCount: number = pdfDoc.getPageCount();
        const author: string[] = pdfDoc.getAuthor()?.split(" ") ?? [];
    
        const content: Content = {
            coverPageNumber: 0,
            customZoomCenterX: 0,
            customZoomCenterY: 936,
            customZoomOrientation: "portrait",
            customZoomPageHeight: 1872,
            customZoomPageWidth: 1404,
            customZoomScale: 1,
            extraMetadata: {},
            fontName: "",
            formatVersion: 1,
            lineHeight: -1,
            margins: 125,
            orientation: "portrait",
            tags: [],
            textAlignment: "justify",
            textScale: 1,
            zoomMode: "bestFit",
            pageTags: [],
            documentMetadata: {
                authors: author
            },
            fileType: path.extname(filepath).slice(1),
            originalPageCount: pageCount,
            pageCount: pageCount,
            pages: Array.from({ length: pageCount + 1 }, () => v4()),
            redirectionPageMap: Array.from({ length: pageCount + 1 }, (_, index) => index),
            sizeInBytes: -1
        }
        const local = {
            "contentFormatVersion": 1
        }
        const pagedata: string = Array.from({ length: pageCount + 1 }, () => "blank\n").join("")
    
        return {metadata:metadata, content:content, local:local, pagedata:pagedata}
    }
}