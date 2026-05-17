const name = process.argv[2] ?? 'task';
const delay = Number(process.env.TASK_DELAY_MS ?? 25);
setTimeout(() => {
  console.log(`task:${name}:ok`);
}, delay);
