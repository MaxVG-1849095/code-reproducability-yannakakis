select count(*) from yago24, yago10_1, yago17_2, yago17_3, yago10_4, yago0, yago39, yago36, yago44 where yago24.d = yago10_1.d and yago10_1.s = yago17_2.s and yago17_2.d = yago17_3.s and yago17_3.d = yago10_4.s and yago10_4.d = yago0.d and yago0.s = yago39.s and yago39.d = yago36.d and yago36.s = yago44.s;