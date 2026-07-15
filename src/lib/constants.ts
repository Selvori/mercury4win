// mercury4win-linux/src/lib/constants.ts

export const APP_NAME = "Mercury";
export const APP_VERSION = "0.1.0";

// Reader pipeline versions
export const READABILITY_VERSION = 1;
export const MARKDOWN_VERSION = 1;
export const READER_RENDER_VERSION = 1;

// Agent task types
export const TASK_TYPES = ["summary", "translation", "tagging"] as const;

// Detail levels
export const DETAIL_LEVELS = ["brief", "medium", "detailed"] as const;

// Prompt strategies for translation
export const PROMPT_STRATEGIES = ["standard", "hy-mt-optimized"] as const;

// Themes
export const READER_THEMES = ["classic", "paper"] as const;
export const READER_COLOR_SCHEMES = ["light", "dark"] as const;

// Default export directory setting key
export const EXPORT_DIR_KEY = "export_directory";
export const LOCALE_KEY = "locale";
