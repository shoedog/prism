const express = require('express');
const router = express.Router();

function list(req, res) { res.sendStatus(204); }
router.get('/accounts', list);

module.exports = router;
