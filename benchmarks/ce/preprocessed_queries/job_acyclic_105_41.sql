select count(*) from imdb100, imdb126, imdb13, imdb56 where imdb100.d = imdb126.d and imdb126.d = imdb13.s and imdb13.s = imdb56.s;