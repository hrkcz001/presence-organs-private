#!/usr/bin/env node
/**
 * Presence Organ Template (TypeScript)
 * Runs directly on Node 26+ without build steps.
 */
import * as process from 'node:process';

function parseArgs(args: string[]): Record<string, string> {
  const params: Record<string, string> = {};
  for (let i = 0; i < args.length; i++) {
    const arg = args[i];
    if (arg.startsWith('--')) {
      const key = arg.slice(2);
      if (i + 1 < args.length && !args[i + 1].startsWith('--')) {
        params[key] = args[i + 1];
        i++;
      } else {
        params[key] = 'true';
      }
    }
  }
  return params;
}

const args = parseArgs(process.argv.slice(2));
const op = args['tool'] || args['action'] || 'default_action';

console.log(JSON.stringify({
  status: 'ok',
  operation: op,
  params: args,
  timestamp: new Date().toISOString()
}, null, 2));