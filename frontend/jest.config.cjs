module.exports = {
  testEnvironment: 'jsdom',
  setupFilesAfterEnv: ['<rootDir>/jest.setup.ts'],
  moduleNameMapper: { '^@/(.*)$': '<rootDir>/src/$1' },
  transform: {
    '^.+\\.(t|j)sx?$': ['@swc/jest', {
      jsc: { target: 'es2022', parser: { syntax: 'typescript', tsx: true }, transform: { react: { runtime: 'automatic' } } },
      module: { type: 'commonjs' },
    }],
  },
  collectCoverageFrom: [
    'src/lib/api.ts',
    'src/lib/mapPopup.ts',
    'src/components/dashboard/GardenGraph.tsx',
    'src/app/(auth)/login/page.tsx',
    'src/app/(auth)/register/page.tsx',
    'src/app/shipments/page.tsx',
    'src/components/scanner/StockScanner.tsx',
  ],
  coverageProvider: 'v8',
  coverageReporters: ['text', 'json-summary', 'lcov'],
  coverageThreshold: { global: { statements: 80, branches: 80, functions: 80, lines: 80 } },
};
