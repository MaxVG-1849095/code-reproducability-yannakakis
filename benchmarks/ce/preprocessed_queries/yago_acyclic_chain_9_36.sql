select count(*) from yago2_0, yago2_1, yago46_2, yago4, yago58_4, yago17, yago58_6, yago58_7, yago46_8 where yago2_0.s = yago2_1.s and yago2_1.d = yago46_2.d and yago46_2.s = yago4.s and yago4.d = yago58_4.d and yago58_4.s = yago17.s and yago17.d = yago58_6.d and yago58_6.s = yago58_7.d and yago58_7.s = yago46_8.d;