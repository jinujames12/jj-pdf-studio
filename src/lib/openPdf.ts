// Opening a PDF that may need a password: every tool goes through here.
import { errorMessage, NEEDS_PASSWORD, openPdf, unlockSession, type DocInfo } from './api';
import { fileName } from './paths';
import { promptForPassword } from './stores/password.svelte';

/** The user closed the password prompt. Callers drop the file quietly. */
export class PasswordCancelled extends Error {
  constructor() {
    super('Password prompt cancelled.');
  }
}

/** Like `openPdf`, but asks for the password (in a dialog) when the file needs one. */
export async function openPdfAsking(path: string): Promise<DocInfo> {
  try {
    return await openPdf(path);
  } catch (e) {
    if (errorMessage(e) !== NEEDS_PASSWORD) throw e;
  }
  const accepted = await promptForPassword(fileName(path), async (password) => {
    try {
      await unlockSession(path, password);
      return null;
    } catch (e) {
      return errorMessage(e);
    }
  });
  if (!accepted) throw new PasswordCancelled();
  return openPdf(path);
}
