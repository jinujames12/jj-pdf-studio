// The password prompt shown when a protected PDF is opened in any tool.

interface Request {
  /** File name shown in the dialog. */
  name: string;
  /** Returns an error message to show in the dialog, or `null` when the password worked. */
  verify: (password: string) => Promise<string | null>;
  done: (accepted: boolean) => void;
}

let request = $state<Request | null>(null);
// Several files can need a password at once (Merge); ask for them one at a time.
let queue: Promise<unknown> = Promise.resolve();

export const passwordPrompt = {
  get request() {
    return request;
  }
};

/** Asks for the password of `name`. Resolves true once `verify` accepted one, false on cancel. */
export function promptForPassword(name: string, verify: Request['verify']): Promise<boolean> {
  const turn = queue.then(
    () =>
      new Promise<boolean>((resolve) => {
        request = {
          name,
          verify,
          done: (accepted) => {
            request = null;
            resolve(accepted);
          }
        };
      })
  );
  queue = turn;
  return turn;
}
