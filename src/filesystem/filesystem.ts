import fs from "fs";
import path from "path";
import { PDFDocument } from "pdf-lib";
import { v4 } from "uuid";
import { Collection, Content, Metadata } from "./filesystem.types.js";

/**
 * Class that manages the conversion to and from Xochitl file system
 */
export class RM2FileSystem {
  root: Collection;
  filesUUIDs: Set<string>;
  collectionUUIDs: Set<string>;
  metadata: Map<string, Metadata>;

  private graphCollections: Map<string, string[]>;
  private graphFiles: Map<string, string[]>;
  
  /**
   * Initialize the file system.
   * @param metadata - Metadata read from the remarkable tablet using RM2Client
   */
  constructor(metadata: Map<string, Metadata>) {
    this.metadata = metadata;
    this.filesUUIDs = new Set();
    this.collectionUUIDs = new Set();

    // Initialize the directed acyclic graph
    this.graphCollections = new Map();
    this.graphFiles = new Map();

    this.graphCollections.set("", []);
    this.graphFiles.set("", []);
    this.metadata.forEach((data: Metadata, uuid: string) => {
      switch (data.type) {
        case "DocumentType":
          this.filesUUIDs.add(uuid);
          break;
        case "CollectionType":
          this.collectionUUIDs.add(uuid);
          this.graphCollections.set(uuid, []);
          this.graphFiles.set(uuid, []);
          break;
        default:
          throw new Error(`Type ${data.type} is not supported.`);
      }
    });

    // Fill the graph
    this.metadata.forEach((data: Metadata, uuid: string) => {
      switch (data.type) {
        case "DocumentType":
          this.graphFiles.get(data.parent)?.push(uuid);
          break;
        case "CollectionType":
          this.collectionUUIDs.add(uuid);
          this.graphCollections.get(data.parent)?.push(uuid);
          break;
        default:
          throw new Error(`Type ${data.type} is not supported.`);
      }
    });

    // Compute the root of the file system representation
    this.root = this.computeNode("");
  }

  /**
   * Compute the graph node of a Collection using recursive exploration of the graph from Xochitl file system.
   * @param uuid - UUID of the node to generate the representation of
   * @returns The representation of the node as a Collection.
   */
  private computeNode(uuid: string): Collection {
    let node: Collection = {
      name: this.metadata.get(uuid)?.visibleName ?? "",
      uuid: uuid,
      collections: [],
      files: new Map(),
    };

    this.graphFiles.get(uuid)?.forEach((fileUUID) => {
      const fileName: string | undefined =
        this.metadata.get(fileUUID)?.visibleName;
      if (fileName) node.files.set(fileName, fileUUID);
    });

    this.graphCollections
      .get(uuid)
      ?.forEach((collectionUUID) =>
        node.collections.push(this.computeNode(collectionUUID))
      );

    return node;
  }

  /**
   * Produce the associated files
   * @param filepath - File path to the file that need to be uploaded
   * @param collection - UUID of the collection to put the file in. If not specified, file is put at the root
   * @returns The content of all document related files
   */
  static async convert(filepath: string, collection?:string) {
    // Create .metadata
    const metadata: Metadata = {} as Metadata;
    await fs.promises.stat(filepath).then((stats) => {
      metadata.createdTime = Math.floor(stats.ctimeMs).toString();
      metadata.lastModified = Math.floor(stats.mtimeMs).toString();
      metadata.lastOpened = "0";
      metadata.lastOpenedPage = 0;
      metadata.parent = collection ?? "";
      metadata.pinned = false;
      metadata.type = "DocumentType";
      metadata.visibleName = "";
    });

    metadata.visibleName = path.basename(filepath);

    const pdfBuffer = fs.readFileSync(filepath);
    const pdfDoc = await PDFDocument.load(pdfBuffer);
    const pageCount: number = pdfDoc.getPageCount();
    const author: string[] = pdfDoc.getAuthor()?.split(" ") ?? [];

    // Create .content
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
        authors: author,
      },
      fileType: path.extname(filepath).slice(1),
      originalPageCount: pageCount,
      pageCount: pageCount,
      pages: Array.from({ length: pageCount + 1 }, () => v4()),
      redirectionPageMap: Array.from(
        { length: pageCount + 1 },
        (_, index) => index
      ),
      sizeInBytes: -1,
    };

    // Create .local
    const local = {
      contentFormatVersion: 1,
    };

    // Create .pagedata
    const pagedata: string = Array.from(
      { length: pageCount + 1 },
      () => "blank\n"
    ).join("");

    return {
      metadata: metadata,
      content: content,
      local: local,
      pagedata: pagedata,
    };
  }

  /**
   * Get the UUID of a collection based on its path.  
   *
   * @param path - Path of the collection in the following fashion:
   *               dir/subdir/subsubdir      
   * @returns The UUID of the collection. If collection is not found return undefined.
   */
  getCollectionUUID(path:string){
    let uuid = ""
    let currCol = this.root

    // Iterate through collection until it's found
    for (const dir of path.split("//")){
      let found = false;
      for (const subdir of currCol.collections){
        if (subdir.name == dir){
          uuid = subdir.uuid;
          currCol = subdir;
          found = true;
          break;
        }
      }
      if (!found) return undefined;
    }
    return uuid;
  }
}