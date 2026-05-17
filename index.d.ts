import type { Readable, Writable } from "node:stream";

declare namespace runAll {
  interface Result {
    name: string;
    code: number | null | undefined;
  }

  interface Options {
    aggregateOutput?: boolean | undefined;
    arguments?: string[] | undefined;
    continueOnError?: boolean | undefined;
    parallel?: boolean | undefined;
    maxParallel?: number | undefined;
    npmPath?: string | undefined;
    packageConfig?: Record<string, Record<string, unknown>> | null | undefined;
    config?: Record<string, unknown> | null | undefined;
    printLabel?: boolean | undefined;
    printName?: boolean | undefined;
    race?: boolean | undefined;
    silent?: boolean | undefined;
    stdin?: Readable | null | undefined;
    stdout?: Writable | null | undefined;
    stderr?: Writable | null | undefined;
    taskList?: string[] | null | undefined;
  }

  class NpmRunAllError extends Error {
    readonly name: "NpmRunAllError";
    readonly results: Result[];
  }
}

declare function runAll(
  patterns: string | string[],
  options?: runAll.Options
): Promise<runAll.Result[] | null>;

export = runAll;
