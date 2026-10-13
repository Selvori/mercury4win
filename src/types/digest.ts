// mercury4win/src/types/digest.ts

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

export interface DigestOptions {
  include_body: boolean;
  include_note: boolean;
  include_summary: boolean;
  locale?: "en" | "zh-Hans";
}

export interface DigestPreview {
  content: string;
  format: "markdown" | "text";
  missing: { entry_id: number; title: string; field: "body" | "note" | "summary" }[];
}
