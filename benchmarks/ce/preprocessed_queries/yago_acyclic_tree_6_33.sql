select count(*) from yago0_0, yago1, yago3, yago0_3, yago0_4, yago58 where yago0_0.d = yago1.d and yago1.d = yago0_4.d and yago1.s = yago3.s and yago3.d = yago0_3.d and yago0_4.s = yago58.d;