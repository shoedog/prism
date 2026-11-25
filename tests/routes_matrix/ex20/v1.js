const express = require('express');
const router = express.Router();

function users(req, res) { res.sendStatus(204); }
router.get('/users', users);

module.exports = router;
