/** @type {import('prettier').Config} */
const config = {
  // Print width and line formatting
  printWidth: 100,
  tabWidth: 2,
  useTabs: false,
  
  // Punctuation
  semi: true,
  singleQuote: true,
  quoteProps: 'as-needed',
  
  // Trailing commas and brackets
  trailingComma: 'es5',
  bracketSpacing: true,
  bracketSameLine: false,
  
  // Arrow functions
  arrowParens: 'avoid',
  
  // Line endings
  endOfLine: 'lf',
  
  // Embedded language formatting
  embeddedLanguageFormatting: 'auto',
  
  // JSX
  jsxSingleQuote: true,
  
  // Plugin overrides for specific files
  overrides: [
    {
      files: '*.json',
      options: {
        singleQuote: false,
      },
    },
    {
      files: '*.md',
      options: {
        printWidth: 80,
        proseWrap: 'always',
      },
    },
  ],
};

module.exports = config;