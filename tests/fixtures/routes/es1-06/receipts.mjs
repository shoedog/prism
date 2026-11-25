import express from 'express';
export const router = express.Router();
function receipt(req, res) { res.send('receipt'); }
router.get('/receipts', receipt);
