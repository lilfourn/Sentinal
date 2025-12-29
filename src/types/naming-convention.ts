/** A suggested naming convention from the AI */
export interface NamingConvention {
  id: string;
  name: string;           // "Kebab Case"
  description: string;    // "lowercase-words-separated-by-hyphens"
  example: string;        // "invoice-apple-dec24.pdf"
  pattern: string;        // Pattern description for AI to follow
  confidence: number;     // 0-1
  matchingFiles: number;  // Count of files matching this pattern
}

/** Response from naming convention analysis */
export interface NamingConventionSuggestions {
  folderPath: string;
  totalFilesAnalyzed: number;
  suggestions: NamingConvention[];
}
