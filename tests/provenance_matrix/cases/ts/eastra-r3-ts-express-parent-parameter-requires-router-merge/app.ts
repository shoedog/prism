import express from 'express';
const app = express();
const included = express.Router({mergeParams: true});
const isolated = express.Router();
included.get('/', (incoming: express.Request, outgoing) => {
  const shelf = incoming.params.shelf;
  outgoing.json(shelf);
});
isolated.get('/', (incoming: express.Request, outgoing) => {
  const shelf = incoming.params.shelf ?? 'local';
  outgoing.json(shelf);
});
app.use('/shared/:shelf', included);
app.use('/isolated/:shelf', isolated);
