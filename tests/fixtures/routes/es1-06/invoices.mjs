import express from 'express';
export const router = express.Router();
function invoice(req, res) { res.send('invoice'); }
router.get('/invoices', invoice);
