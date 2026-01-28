// Pick a Number API
// TODO: Add authentication middleware
// TODO(security): Validate input ranges

const express = require('express');
const app = express();

// TODO: Move to config file
const PORT = 3000;
const MIN_NUMBER = 1;
const MAX_NUMBER = 100;

let secretNumber = Math.floor(Math.random() * MAX_NUMBER) + MIN_NUMBER;

// TODO(perf): Add caching for repeated guesses
app.get('/guess/:number', (req, res) => {
  const guess = parseInt(req.params.number);
  
  // TODO: Add rate limiting
  if (isNaN(guess)) {
    return res.status(400).json({ error: 'Invalid number' });
  }
  
  if (guess === secretNumber) {
    res.json({ result: 'correct', message: 'You won!' });
    secretNumber = Math.floor(Math.random() * MAX_NUMBER) + MIN_NUMBER;
  } else if (guess < secretNumber) {
    res.json({ result: 'higher', message: 'Go higher!' });
  } else {
    res.json({ result: 'lower', message: 'Go lower!' });
  }
});

// TODO: Add health check endpoint
app.listen(PORT, () => {
  console.log(`Server running on port ${PORT}`);
});
