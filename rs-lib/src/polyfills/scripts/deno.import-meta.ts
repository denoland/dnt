declare global {
  interface ImportMeta {
    /** A flag that indicates if the current module is the main module that was
     * called when starting the program under Deno.
     *
     * ```ts
     * if (import.meta.main) {
     *   // this was loaded as the main module, maybe do some bootstrapping
     * }
     * ```
     */
    main: boolean;

    /** A function that returns resolved specifier as if it would be imported
     * using `import(specifier)`.
     *
     * ```ts
     * console.log(import.meta.resolve("./foo.js"));
     * // file:///dev/foo.js
     * ```
     */
    // @ts-ignore override
    resolve: (specifier: string) => string;
  }
}

export {}