select count(*) from yago0, yago3, yago23, yago22, yago46 yago46_4, yago46 yago46_5, yago58 yago58_6, yago58 yago58_7, yago32 where yago0.d = yago3.d and yago3.s = yago23.s and yago23.d = yago22.d and yago22.s = yago46_4.s and yago46_4.d = yago46_5.d and yago46_5.s = yago58_6.d and yago58_6.s = yago58_7.d and yago58_7.s = yago32.s;