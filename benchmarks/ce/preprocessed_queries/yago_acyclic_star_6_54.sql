select count(*) from yago36_0, yago12_1, yago36_2, yago12_3, yago2_4, yago2_5 where yago36_0.s = yago12_1.s and yago12_1.s = yago36_2.s and yago36_2.s = yago12_3.s and yago12_3.s = yago2_4.d and yago2_4.d = yago2_5.d;