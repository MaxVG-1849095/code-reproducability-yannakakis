select count(*) from yago13_0, yago13_1, yago5, yago36, yago35, yago17 where yago13_0.s = yago13_1.s and yago13_1.d = yago5.d and yago5.s = yago36.s and yago36.d = yago35.d and yago35.s = yago17.d;