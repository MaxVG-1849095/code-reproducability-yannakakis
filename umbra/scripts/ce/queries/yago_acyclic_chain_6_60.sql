select count(*) from yago46 yago46_0, yago46 yago46_1, yago58, yago4 yago4_3, yago4 yago4_4, yago4 yago4_5 where yago46_0.d = yago46_1.d and yago46_1.s = yago58.s and yago58.d = yago4_3.s and yago4_3.d = yago4_4.s and yago4_4.d = yago4_5.d;