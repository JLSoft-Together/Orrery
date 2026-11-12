export interface WebFile {
  type: string;
  cache: "immutable" | "revalidate";
  etag: string;
  data: string;
}

export interface WebBundle {
  hash: string;
  files: Record<string, WebFile>;
}
