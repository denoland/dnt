export function read_file_sync(file_path) {
  return Deno.readTextFileSync(file_path);
}
