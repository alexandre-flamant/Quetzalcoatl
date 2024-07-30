/**
 * Contnent of a .metadata file
 */
export type Metadata = {
  createdTime: string;
  lastModified: string;
  lastOpened: string;
  lastOpenedPage: number;
  parent: string; // Parent UUID
  pinned: boolean;
  type: string;
  visibleName: string;
};

/**
 * Content of the .content file
 */
export type Content = {
  coverPageNumber: number;
  customZoomCenterX: number;
  customZoomCenterY: number;
  customZoomOrientation: "portrait" | "landscape";
  customZoomPageHeight: number;
  customZoomPageWidth: number;
  customZoomScale: number;
  documentMetadata: { authors: string[] }; //
  extraMetadata: {};
  fileType: string;
  fontName: string;
  formatVersion: number;
  lineHeight: number;
  margins: number;
  orientation: "portrait" | "landscape";
  originalPageCount: number; //
  pageCount: number; //
  pageTags: [];
  pages: string[]; //
  redirectionPageMap: number[]; //
  sizeInBytes: number; //
  tags: [];
  textAlignment: "justify" | string;
  textScale: number;
  zoomMode: string;
};

/**
 * Describe a Collection in the Xochitl file system.
 * 
 * @member name - Name of the collection
 * @member uuid - UUID associated to the collection
 * @member collection - List of sub-collection within the collection
 * @member files - Map that link file within the collection name as key to the corresponding UUID.
 */
export type Collection = {
  name: string;
  uuid: string;
  collections: Collection[];
  files: Map<string, string>;
};
