select count(*) from yago2 yago2_0, yago2 yago2_1, yago58, yago22, yago13, yago36 where yago2_0.s = yago2_1.s and yago2_1.d = yago58.d and yago58.s = yago22.s and yago22.d = yago13.d and yago13.s = yago36.s;