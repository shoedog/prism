const express = require('express');

const router = express.Router();
function audit(req, res) { res.json({ events: [] }); }
router.get('/audit', audit);

module.exports = router;
