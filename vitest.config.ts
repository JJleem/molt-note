import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    // 제품 테스트만 본다.
    // tools/loop-runtime 은 Loop Runtime 자체의 회귀 테스트이며 node:test로 돌린다
    // (`node --test "tools/loop-runtime/test/*.test.mjs"`). 제품 Gate가 소유하지 않는다.
    include: ['tests/**/*.test.ts', 'src/**/*.test.{ts,tsx}'],
    exclude: ['node_modules', 'dist', 'src-tauri', 'tools', '.loop-local'],
  },
});
