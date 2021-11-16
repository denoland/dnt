import { assertRejects } from "./test.deps.ts";
import { runCommand } from "./utils.ts";

Deno.test({
  name: "should error when command doesn't exist",
  ignore: Deno.build.os === "windows",
  async fn() {
    const commandName = "somenonexistentcommandforsure";
    await assertRejects(
      () =>
        runCommand({
          cmd: [commandName],
          cwd: Deno.cwd(),
        }),
      Error,
      `Could not find command '${commandName}'. Ensure it is available on the path.`,
    );
  },
});
