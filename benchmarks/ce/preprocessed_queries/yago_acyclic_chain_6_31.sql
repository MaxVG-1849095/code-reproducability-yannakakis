select count(*) from yago2_0, yago2_1, yago65, yago17_3, yago17_4, yago17_5 where yago2_0.s = yago2_1.s and yago2_1.d = yago65.d and yago65.s = yago17_3.d and yago17_3.s = yago17_4.s and yago17_4.d = yago17_5.d;