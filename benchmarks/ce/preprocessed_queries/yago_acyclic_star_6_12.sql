select count(*) from yago36_0, yago36_1, yago36_2, yago50, yago43, yago2 where yago36_0.s = yago36_1.s and yago36_1.s = yago36_2.s and yago36_2.s = yago50.s and yago50.s = yago43.s and yago43.s = yago2.d;