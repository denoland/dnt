import { join } from "https://deno.land/std@0.142.0/node/path.ts";
import fs from "https://deno.land/std@0.142.0/node/fs.ts";

console.log(join("test", "other"));
fs.writeFileSync("test.txt", "test");
