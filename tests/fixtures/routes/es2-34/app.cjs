const express = require('express');
const app = express();
const router = express.Router();

function show(req, res) { res.json([]); }
router.get('/catalog', show);

app.use('/tenant-01', router); app.use('/tenant-02', router);
app.use('/tenant-03', router); app.use('/tenant-04', router);
app.use('/tenant-05', router); app.use('/tenant-06', router);
app.use('/tenant-07', router); app.use('/tenant-08', router);
app.use('/tenant-09', router); app.use('/tenant-10', router);
app.use('/tenant-11', router); app.use('/tenant-12', router);
app.use('/tenant-13', router); app.use('/tenant-14', router);
app.use('/tenant-15', router); app.use('/tenant-16', router);
app.use('/tenant-17', router); app.use('/tenant-18', router);
app.use('/tenant-19', router); app.use('/tenant-20', router);
app.use('/tenant-21', router); app.use('/tenant-22', router);
app.use('/tenant-23', router); app.use('/tenant-24', router);
app.use('/tenant-25', router); app.use('/tenant-26', router);
app.use('/tenant-27', router); app.use('/tenant-28', router);
app.use('/tenant-29', router); app.use('/tenant-30', router);
app.use('/tenant-31', router); app.use('/tenant-32', router);
app.use('/tenant-33', router); app.use('/tenant-34', router);
app.use('/tenant-35', router); app.use('/tenant-36', router);
app.use('/tenant-37', router); app.use('/tenant-38', router);
app.use('/tenant-39', router); app.use('/tenant-40', router);
app.use('/tenant-41', router); app.use('/tenant-42', router);
app.use('/tenant-43', router); app.use('/tenant-44', router);
app.use('/tenant-45', router); app.use('/tenant-46', router);
app.use('/tenant-47', router); app.use('/tenant-48', router);
app.use('/tenant-49', router); app.use('/tenant-50', router);
app.use('/tenant-51', router); app.use('/tenant-52', router);
app.use('/tenant-53', router); app.use('/tenant-54', router);
app.use('/tenant-55', router); app.use('/tenant-56', router);
app.use('/tenant-57', router); app.use('/tenant-58', router);
app.use('/tenant-59', router); app.use('/tenant-60', router);
app.use('/tenant-61', router); app.use('/tenant-62', router);
app.use('/tenant-63', router); app.use('/tenant-64', router);
app.use('/tenant-65', router);

module.exports = app;
