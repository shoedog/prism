const express = require('express');
const app = express();

function removeDraft(req, res) { res.sendStatus(204); }
app.del('/drafts/:id', removeDraft);

module.exports = app;
