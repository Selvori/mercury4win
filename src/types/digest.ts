// mercury4win-linux/src/types/digest.ts

export interface DigestTemplate {
  name: string;
  display_name: string;
  description: string;
  output_format: "markdown" | "text" | "html";
}

export interface DigestExport {
  content: string;
  format: string;
}
